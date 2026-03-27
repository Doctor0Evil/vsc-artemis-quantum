@file:Suppress("unused", "MemberVisibilityCanBePrivate")
package artemis.econet.corridor.android

import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.IBinder
import android.os.Handler
import android.os.Looper
import android.util.Log
import kotlinx.coroutines.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.concurrent.atomic.AtomicBoolean
import java.util.zip.GZIPInputStream
import java.util.zip.GZIPOutputStream

class CorridorSynergyService : Service() {

    companion object {
        private const val TAG = "ArtemisCorridorService"
        private const val API_BASE = "https://api.artemis.econet.io/v1"
        private const val LEDGER_ENDPOINT = "https://ledger.artemis.econet.io/commit"
        private const val KERNEL_HASH = "0x21d4a9c73f58e2b19a40c6d8e3f7b5a1c9e2047f3a8d6b2c51e07f49a3c6d8e2b7f1049c3e8a6d2f5b1c7e9a304f6d1c9"
        private const val CONTRACT_VERSION = "1.0.0"
        private const val KARMA_NAMESPACE = "artemis.econet"
        private const val ACTION_RADIUS_LAT_MIN = 33.44
        private const val ACTION_RADIUS_LAT_MAX = 33.45
        private const val ACTION_RADIUS_LON_MIN = -112.08
        private const val ACTION_RADIUS_LON_MAX = -112.07
        private const val REQUEST_TIMEOUT_MS = 30000L
        private const val MAX_RETRY_COUNT = 3
        private const val SHARD_COMPRESSION_LEVEL = 6

        init {
            System.loadLibrary("artemis_corridor_kernel")
        }

        @JvmStatic external fun nativeKernelCreate(configJson: String): Long
        @JvmStatic external fun nativeKernelDestroy(handle: Long)
        @JvmStatic external fun nativeKernelValidateConfig(handle: Long): Int
        @JvmStatic external fun nativeKernelRankInterventions(handle: Long, corridorsJson: String, interventionsJson: String, maxResults: Int): String
        @JvmStatic external fun nativeKernelGetVersion(): String
        @JvmStatic external fun nativeKernelGetHash(): String
    }

    private val binder = LocalBinder()
    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private val isRunning = AtomicBoolean(false)
    private val operationMutex = Mutex()
    private val mainHandler = Handler(Looper.getMainLooper())
    private var kernelHandle: Long = 0L
    private var karmaToken: String? = null
    private var identityDid: String = "did:econet:artemis:corridor:synergy:v1"
    private var configWeights = doubleArrayOf(0.18, 0.22, 0.18, 0.24, 0.18)
    private var maxCostPerCorridor = 5000000.0
    private var minKerK = 0.85
    private var maxKerR = 0.20
    private var enforceMonotonicity = true

    data class CorridorState(
        val corridorId: String, val lat: Double, val lon: Double,
        val ecoScores: DoubleArray, val K: Double, val E: Double, val R: Double,
        val timestampUnix: Long, val evidenceHex: String
    )

    data class InterventionDef(
        val interventionId: String, val interventionType: String,
        val responseCoeffs: DoubleArray, val costUsd: Double,
        val energyKwh: Double, val landM2: Double,
        val kerDelta: DoubleArray, val linkedEvidenceHex: String
    )

    data class RankedIntervention(
        val corridorId: String, val interventionId: String,
        val synergyGain: Double, val costUsd: Double,
        val efficiencyRatio: Double, val projectedKer: DoubleArray,
        val safetyFlags: Int
    )

    data class KernelConfig(
        val weights: DoubleArray, val maxCostPerCorridor: Double,
        val minKerK: Double, val maxKerR: Double,
        val enforceMonotonicity: Boolean, val configVersion: String,
        val validFromUnix: Long, val validUntilUnix: Long
    )

    interface ServiceCallback {
        fun onRankingComplete(results: List<RankedIntervention>)
        fun onRankingError(error: String)
        fun onLedgerCommitSuccess(hash: String)
        fun onLedgerCommitError(error: String)
        fun onKarmaUpdated(newKarma: Double)
    }

    private val callbacks = mutableListOf<ServiceCallback>()

    inner class LocalBinder : Binder() {
        fun getService(): CorridorSynergyService = this@CorridorSynergyService
    }

    override fun onBind(intent: Intent?): IBinder = binder

    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "Artemis Corridor Synergy Service created - Kernel: $KERNEL_HASH")
    }

    override fun onDestroy() {
        serviceScope.cancel()
        if (kernelHandle != 0L) {
            nativeKernelDestroy(kernelHandle)
            kernelHandle = 0L
        }
        super.onDestroy()
        Log.i(TAG, "Artemis Corridor Synergy Service destroyed")
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        intent?.getStringExtra("KARMA_TOKEN")?.let { karmaToken = it }
        intent?.getStringExtra("IDENTITY_DID")?.let { identityDid = it }
        if (!isRunning.getAndSet(true)) {
            serviceScope.launch { initializeKernel() }
        }
        return START_STICKY
    }

    fun registerCallback(callback: ServiceCallback) {
        synchronized(callbacks) { callbacks.add(callback) }
    }

    fun unregisterCallback(callback: ServiceCallback) {
        synchronized(callbacks) { callbacks.remove(callback) }
    }

    private suspend fun initializeKernel() = operationMutex.withLock {
        try {
            val config = KernelConfig(
                weights = configWeights,
                maxCostPerCorridor = maxCostPerCorridor,
                minKerK = minKerK,
                maxKerR = maxKerR,
                enforceMonotonicity = enforceMonotonicity,
                configVersion = "phoenix-2026-v1",
                validFromUnix = 1735689600,
                validUntilUnix = 1767225600
            )
            val configJson = JSONObject().apply {
                put("weights", JSONArray(config.weights))
                put("max_cost_per_corridor", config.maxCostPerCorridor)
                put("min_KER_K", config.minKerK)
                put("max_KER_R", config.maxKerR)
                put("enforce_monotonicity", config.enforceMonotonicity)
                put("config_version", config.configVersion)
                put("valid_from_unix", config.validFromUnix)
                put("valid_until_unix", config.validUntilUnix)
            }.toString()
            kernelHandle = nativeKernelCreate(configJson)
            if (kernelHandle == 0L) throw IllegalStateException("Kernel creation failed")
            val status = nativeKernelValidateConfig(kernelHandle)
            if (status != 0) throw IllegalStateException("Kernel config validation failed: $status")
            Log.i(TAG, "Kernel initialized - Version: ${nativeKernelGetVersion()}")
        } catch (e: Exception) {
            Log.e(TAG, "Kernel initialization failed", e)
            notifyError("Kernel initialization: ${e.message}")
        }
    }

    fun loadCorridorShards(filePath: String): List<CorridorState> {
        val file = File(filePath)
        if (!file.exists()) throw IllegalArgumentException("Shard file not found: $filePath")
        val corridors = mutableListOf<CorridorState>()
        FileInputStream(file).use { fis ->
            GZIPInputStream(fis).bufferedReader().useLines { lines ->
                val header = lines.firstOrNull() ?: return@useLines
                for (line in lines) {
                    val fields = line.split(",")
                    if (fields.size >= 13) {
                        val lat = fields[1].toDoubleOrNull() ?: continue
                        val lon = fields[2].toDoubleOrNull() ?: continue
                        if (!validateGeographicBounds(lat, lon)) continue
                        val ecoScores = doubleArrayOf(
                            fields[3].toDoubleOrNull() ?: 0.0,
                            fields[4].toDoubleOrNull() ?: 0.0,
                            fields[5].toDoubleOrNull() ?: 0.0,
                            fields[6].toDoubleOrNull() ?: 0.0,
                            fields[7].toDoubleOrNull() ?: 0.0
                        )
                        if (!validateEcoScores(ecoScores)) continue
                        corridors.add(CorridorState(
                            corridorId = fields[0], lat = lat, lon = lon,
                            ecoScores = ecoScores,
                            K = fields[8].toDoubleOrNull() ?: 0.0,
                            E = fields[9].toDoubleOrNull() ?: 0.0,
                            R = fields[10].toDoubleOrNull() ?: 0.0,
                            timestampUnix = fields[11].toLongOrNull() ?: System.currentTimeMillis() / 1000,
                            evidenceHex = fields[12]
                        ))
                    }
                }
            }
        }
        return corridors
    }

    fun loadInterventionShards(filePath: String): List<InterventionDef> {
        val file = File(filePath)
        if (!file.exists()) throw IllegalArgumentException("Shard file not found: $filePath")
        val interventions = mutableListOf<InterventionDef>()
        FileInputStream(file).use { fis ->
            GZIPInputStream(fis).bufferedReader().useLines { lines ->
                val header = lines.firstOrNull() ?: return@useLines
                for (line in lines) {
                    val fields = line.split(",")
                    if (fields.size >= 14) {
                        val costUsd = fields[7].toDoubleOrNull() ?: continue
                        if (costUsd <= 0.0) continue
                        val responseCoeffs = doubleArrayOf(
                            fields[2].toDoubleOrNull() ?: 0.0,
                            fields[3].toDoubleOrNull() ?: 0.0,
                            fields[4].toDoubleOrNull() ?: 0.0,
                            fields[5].toDoubleOrNull() ?: 0.0,
                            fields[6].toDoubleOrNull() ?: 0.0
                        )
                        if (!validateEcoScores(responseCoeffs)) continue
                        interventions.add(InterventionDef(
                            interventionId = fields[0], interventionType = fields[1],
                            responseCoeffs = responseCoeffs, costUsd = costUsd,
                            energyKwh = fields[8].toDoubleOrNull() ?: 0.0,
                            landM2 = fields[9].toDoubleOrNull() ?: 0.0,
                            kerDelta = doubleArrayOf(
                                fields[10].toDoubleOrNull() ?: 0.0,
                                fields[11].toDoubleOrNull() ?: 0.0,
                                fields[12].toDoubleOrNull() ?: 0.0
                            ),
                            linkedEvidenceHex = fields[13]
                        ))
                    }
                }
            }
        }
        return interventions
    }

    fun rankInterventions(corridors: List<CorridorState>, interventions: List<InterventionDef>, maxResults: Int = 10) {
        if (kernelHandle == 0L) { notifyError("Kernel not initialized"); return }
        serviceScope.launch {
            try {
                val corridorsJson = JSONArray().apply {
                    corridors.forEach { c ->
                        put(JSONObject().apply {
                            put("corridor_id", c.corridorId)
                            put("lat", c.lat)
                            put("lon", c.lon)
                            put("eco_scores", JSONArray(c.ecoScores))
                            put("K", c.K)
                            put("E", c.E)
                            put("R", c.R)
                            put("timestamp_unix", c.timestampUnix)
                            put("evidence_hex", c.evidenceHex)
                        })
                    }
                }.toString()
                val interventionsJson = JSONArray().apply {
                    interventions.forEach { i ->
                        put(JSONObject().apply {
                            put("intervention_id", i.interventionId)
                            put("intervention_type", i.interventionType)
                            put("response_coeffs", JSONArray(i.responseCoeffs))
                            put("cost_usd", i.costUsd)
                            put("energy_kwh", i.energyKwh)
                            put("land_m2", i.landM2)
                            put("ker_delta", JSONArray(i.kerDelta))
                            put("linked_evidence_hex", i.linkedEvidenceHex)
                        })
                    }
                }.toString()
                val resultsJson = nativeKernelRankInterventions(kernelHandle, corridorsJson, interventionsJson, maxResults)
                val results = parseRankedInterventions(resultsJson)
                mainHandler.post { notifyRankingComplete(results) }
                serviceScope.launch { submitToLedger(results) }
            } catch (e: Exception) {
                Log.e(TAG, "Ranking failed", e)
                mainHandler.post { notifyError("Ranking: ${e.message}") }
            }
        }
    }

    private fun parseRankedInterventions(jsonString: String): List<RankedIntervention> {
        val results = mutableListOf<RankedIntervention>()
        val jsonArray = JSONArray(jsonString)
        for (i in 0 until jsonArray.length()) {
            val obj = jsonArray.getJSONObject(i)
            results.add(RankedIntervention(
                corridorId = obj.getString("corridor_id"),
                interventionId = obj.getString("intervention_id"),
                synergyGain = obj.getDouble("synergy_gain"),
                costUsd = obj.getDouble("cost_usd"),
                efficiencyRatio = obj.getDouble("efficiency_ratio"),
                projectedKer = doubleArrayOf(
                    obj.getJSONArray("projected_KER").getDouble(0),
                    obj.getJSONArray("projected_KER").getDouble(1),
                    obj.getJSONArray("projected_KER").getDouble(2)
                ),
                safetyFlags = obj.getInt("safety_flags")
            ))
        }
        return results
    }

    private suspend fun submitToLedger(results: List<RankedIntervention>) = operationMutex.withLock {
        try {
            val shardData = JSONArray().apply {
                results.forEach { r ->
                    put(JSONObject().apply {
                        put("corridor_id", r.corridorId)
                        put("intervention_id", r.interventionId)
                        put("synergy_gain", r.synergyGain)
                        put("cost_usd", r.costUsd)
                        put("efficiency_ratio", r.efficiencyRatio)
                        put("projected_KER", JSONArray(r.projectedKer))
                        put("safety_flags", r.safetyFlags)
                    })
                }
            }.toString()
            val shardHash = computeSha3Hash(shardData)
            val payload = JSONObject().apply {
                put("shard_hash", shardHash)
                put("shard_type", "ArtemisCorridorPlans2026v1")
                put("identity_did", identityDid)
                put("karma_namespace", KARMA_NAMESPACE)
                put("timestamp_unix", System.currentTimeMillis() / 1000)
                put("config_version", "phoenix-2026-v1")
                put("contract_version", CONTRACT_VERSION)
                put("kernel_hash", KERNEL_HASH)
            }.toString()
            val response = httpPost(LEDGER_ENDPOINT, payload)
            if (response.first == 200) {
                Log.i(TAG, "Ledger commit success: $shardHash")
                mainHandler.post { notifyLedgerSuccess(shardHash) }
                serviceScope.launch { updateKarma(results) }
            } else {
                throw Exception("Ledger commit failed: ${response.first}")
            }
        } catch (e: Exception) {
            Log.e(TAG, "Ledger commit failed", e)
            mainHandler.post { notifyError("Ledger: ${e.message}") }
        }
    }

    private suspend fun updateKarma(results: List<RankedIntervention>) {
        try {
            val avgEfficiency = results.map { it.efficiencyRatio }.average()
            val ceimGain = avgEfficiency * 0.01
            val payload = JSONObject().apply {
                put("identity_did", identityDid)
                put("karma_namespace", KARMA_NAMESPACE)
                put("ceim_gain", ceimGain)
                put("action_type", "corridor_synergy_ranking")
                put("timestamp_unix", System.currentTimeMillis() / 1000)
                put("result_count", results.size)
            }.toString()
            val headers = mapOf("Content-Type" to "application/json")
            karmaToken?.let { headers.plus("Authorization" to "Bearer $it") }
            val response = httpPost("$API_BASE/karma/update", payload, headers)
            if (response.first == 200) {
                val newKarma = JSONObject(response.second).getDouble("currentKarma")
                mainHandler.post { notifyKarmaUpdated(newKarma) }
            }
        } catch (e: Exception) {
            Log.w(TAG, "Karma update failed", e)
        }
    }

    private fun validateGeographicBounds(lat: Double, lon: Double): Boolean {
        if (lat < ACTION_RADIUS_LAT_MIN || lat > ACTION_RADIUS_LAT_MAX) return false
        if (lon < ACTION_RADIUS_LON_MIN || lon > ACTION_RADIUS_LON_MAX) return false
        return true
    }

    private fun validateEcoScores(scores: DoubleArray): Boolean {
        return scores.all { it >= 0.0 && it <= 1.0 }
    }

    private fun computeSha3Hash(data: String): String {
        val digest = MessageDigest.getInstance("SHA3-256")
        val hashBytes = digest.digest(data.toByteArray())
        return "0x" + hashBytes.joinToString("") { "%02x".format(it) }
    }

    private suspend fun httpPost(urlString: String, payload: String, headers: Map<String, String> = emptyMap()): Pair<Int, String> {
        var retryCount = 0
        while (retryCount < MAX_RETRY_COUNT) {
            try {
                val url = URL(urlString)
                val connection = url.openConnection() as HttpURLConnection
                connection.requestMethod = "POST"
                connection.connectTimeout = REQUEST_TIMEOUT_MS.toInt()
                connection.readTimeout = REQUEST_TIMEOUT_MS.toInt()
                connection.doOutput = true
                connection.setRequestProperty("Content-Type", "application/json")
                headers.forEach { connection.setRequestProperty(it.key, it.value) }
                GZIPOutputStream(connection.outputStream).use { os ->
                    os.write(payload.toByteArray())
                }
                val responseCode = connection.responseCode
                val responseBody = if (responseCode == 200) {
                    connection.inputStream.bufferedReader().use { it.readText() }
                } else {
                    connection.errorStream?.bufferedReader()?.use { it.readText() } ?: ""
                }
                connection.disconnect()
                return Pair(responseCode, responseBody)
            } catch (e: Exception) {
                retryCount++
                if (retryCount >= MAX_RETRY_COUNT) throw e
                delay(1000L * retryCount)
            }
        }
        throw Exception("Max retries exceeded")
    }

    private fun notifyRankingComplete(results: List<RankedIntervention>) {
        synchronized(callbacks) { callbacks.forEach { it.onRankingComplete(results) } }
    }

    private fun notifyError(error: String) {
        synchronized(callbacks) { callbacks.forEach { it.onRankingError(error) } }
    }

    private fun notifyLedgerSuccess(hash: String) {
        synchronized(callbacks) { callbacks.forEach { it.onLedgerCommitSuccess(hash) } }
    }

    private fun notifyKarmaUpdated(newKarma: Double) {
        synchronized(callbacks) { callbacks.forEach { it.onKarmaUpdated(newKarma) } }
    }

    fun getKernelVersion(): String = nativeKernelGetVersion()
    fun getKernelHash(): String = nativeKernelGetHash()
    fun isKernelInitialized(): Boolean = kernelHandle != 0L
}
