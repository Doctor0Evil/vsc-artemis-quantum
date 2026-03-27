"use strict";
const ARTEMIS_CONFIG = Object.freeze({
    API_BASE: "https://api.artemis.econet.io/v1",
    LEDGER_ENDPOINT: "https://ledger.artemis.econet.io/commit",
    KERNEL_HASH: "0x21d4a9c73f58e2b19a40c6d8e3f7b5a1c9e2047f3a8d6b2c51e07f49a3c6d8e2b7f1049c3e8a6d2f5b1c7e9a304f6d1c9",
    CONTRACT_VERSION: "1.0.0",
    KARMA_NAMESPACE: "artemis.econet",
    ACTION_RADIUS: { latMin: 33.44, latMax: 33.45, lonMin: -112.08, lonMax: -112.07 },
    REQUEST_TIMEOUT: 30000,
    MAX_RETRY_COUNT: 3,
    REPOSITORY: "https://github.com/Doctor0Evil/vsc-artemis-quantum"
});

class ArtemisDashboard {
    constructor(config = {}) {
        this.apiBase = config.apiBase || ARTEMIS_CONFIG.API_BASE;
        this.ledgerEndpoint = config.ledgerEndpoint || ARTEMIS_CONFIG.LEDGER_ENDPOINT;
        this.karmaToken = config.karmaToken || null;
        this.identityDid = config.identityDid || "did:econet:artemis:corridor:synergy:v1";
        this.containerId = config.containerId || "artemis-dashboard";
        this.corridors = [];
        this.interventions = [];
        this.rankedResults = [];
        this.eventListeners = new Map();
        this.init();
    }

    init() {
        this.validateConfig();
        this.setupEventListeners();
        this.render();
        console.log(`Artemis Dashboard initialized - Kernel: ${ARTEMIS_CONFIG.KERNEL_HASH}`);
    }

    validateConfig() {
        if (!this.apiBase || typeof this.apiBase !== "string") throw new Error("Invalid API base URL");
        if (!this.ledgerEndpoint || typeof this.ledgerEndpoint !== "string") throw new Error("Invalid ledger endpoint");
        const bounds = ARTEMIS_CONFIG.ACTION_RADIUS;
        if (bounds.latMin >= bounds.latMax || bounds.lonMin >= bounds.lonMax) throw new Error("Invalid action radius bounds");
    }

    setupEventListeners() {
        document.addEventListener("DOMContentLoaded", () => this.onDOMContentLoaded());
        window.addEventListener("resize", () => this.onWindowResize());
    }

    onDOMContentLoaded() {
        this.loadCorridorShards();
        this.loadInterventionShards();
        this.updateKernelStatus();
    }

    onWindowResize() {
        this.renderCorridorMap();
        this.renderResultsTable();
    }

    async loadCorridorShards(filePath = "data/SmartCorridorEcoImpact2026v1.csv") {
        try {
            const response = await this.httpGet(filePath);
            this.corridors = this.parseCorridorCSV(response);
            this.emit("corridorsLoaded", this.corridors);
            this.renderCorridorList();
        } catch (error) {
            console.error("Failed to load corridor shards:", error);
            this.emit("error", { type: "corridor_load", message: error.message });
        }
    }

    async loadInterventionShards(filePath = "data/SmartCorridorInterventions2026v1.csv") {
        try {
            const response = await this.httpGet(filePath);
            this.interventions = this.parseInterventionCSV(response);
            this.emit("interventionsLoaded", this.interventions);
            this.renderInterventionList();
        } catch (error) {
            console.error("Failed to load intervention shards:", error);
            this.emit("error", { type: "intervention_load", message: error.message });
        }
    }

    parseCorridorCSV(csvText) {
        const lines = csvText.trim().split("\n");
        const header = lines[0].split(",");
        const corridors = [];
        for (let i = 1; i < lines.length; i++) {
            const fields = lines[i].split(",");
            if (fields.length < 13) continue;
            const lat = parseFloat(fields[1]);
            const lon = parseFloat(fields[2]);
            if (!this.validateGeographicBounds(lat, lon)) continue;
            const ecoScores = [parseFloat(fields[3]), parseFloat(fields[4]), parseFloat(fields[5]), parseFloat(fields[6]), parseFloat(fields[7])];
            if (!this.validateEcoScores(ecoScores)) continue;
            corridors.push({
                corridorId: fields[0], lat, lon, ecoScores,
                K: parseFloat(fields[8]) || 0.0,
                E: parseFloat(fields[9]) || 0.0,
                R: parseFloat(fields[10]) || 0.0,
                timestampUnix: parseInt(fields[11]) || Date.now() / 1000,
                evidenceHex: fields[12] || ""
            });
        }
        return corridors;
    }

    parseInterventionCSV(csvText) {
        const lines = csvText.trim().split("\n");
        const header = lines[0].split(",");
        const interventions = [];
        for (let i = 1; i < lines.length; i++) {
            const fields = lines[i].split(",");
            if (fields.length < 14) continue;
            const costUsd = parseFloat(fields[7]);
            if (costUsd <= 0.0) continue;
            const responseCoeffs = [parseFloat(fields[2]), parseFloat(fields[3]), parseFloat(fields[4]), parseFloat(fields[5]), parseFloat(fields[6])];
            if (!this.validateEcoScores(responseCoeffs)) continue;
            interventions.push({
                interventionId: fields[0], interventionType: fields[1], responseCoeffs, costUsd,
                energyKwh: parseFloat(fields[8]) || 0.0,
                landM2: parseFloat(fields[9]) || 0.0,
                kerDelta: [parseFloat(fields[10]) || 0.0, parseFloat(fields[11]) || 0.0, parseFloat(fields[12]) || 0.0],
                linkedEvidenceHex: fields[13] || ""
            });
        }
        return interventions;
    }

    validateGeographicBounds(lat, lon) {
        const bounds = ARTEMIS_CONFIG.ACTION_RADIUS;
        return lat >= bounds.latMin && lat <= bounds.latMax && lon >= bounds.lonMin && lon <= bounds.lonMax;
    }

    validateEcoScores(scores) {
        return scores.every(s => typeof s === "number" && isFinite(s) && s >= 0.0 && s <= 1.0);
    }

    async rankInterventions(maxResults = 10) {
        if (this.corridors.length === 0 || this.interventions.length === 0) {
            throw new Error("No corridors or interventions loaded");
        }
        try {
            const payload = {
                corridors: this.corridors,
                interventions: this.interventions,
                maxResults,
                config: {
                    weights: [0.18, 0.22, 0.18, 0.24, 0.18],
                    maxCostPerCorridor: 5000000.0,
                    minKerK: 0.85,
                    maxKerR: 0.20,
                    enforceMonotonicity: true,
                    configVersion: "phoenix-2026-v1"
                }
            };
            const response = await this.httpPost(`${this.apiBase}/interventions/ranked`, payload);
            this.rankedResults = response.results || [];
            this.emit("rankingComplete", this.rankedResults);
            this.renderResultsTable();
            await this.submitToLedger();
            return this.rankedResults;
        } catch (error) {
            console.error("Ranking failed:", error);
            this.emit("error", { type: "ranking", message: error.message });
            throw error;
        }
    }

    async submitToLedger() {
        if (this.rankedResults.length === 0) return;
        try {
            const shardData = JSON.stringify(this.rankedResults);
            const shardHash = await this.computeSha3Hash(shardData);
            const payload = {
                shard_hash: shardHash,
                shard_type: "ArtemisCorridorPlans2026v1",
                identity_did: this.identityDid,
                karma_namespace: ARTEMIS_CONFIG.KARMA_NAMESPACE,
                timestamp_unix: Math.floor(Date.now() / 1000),
                config_version: "phoenix-2026-v1",
                contract_version: ARTEMIS_CONFIG.CONTRACT_VERSION,
                kernel_hash: ARTEMIS_CONFIG.KERNEL_HASH
            };
            const response = await this.httpPost(this.ledgerEndpoint, payload);
            this.emit("ledgerCommit", { hash: shardHash, success: true });
            console.log(`Ledger commit success: ${shardHash}`);
            await this.updateKarma();
        } catch (error) {
            console.error("Ledger commit failed:", error);
            this.emit("error", { type: "ledger", message: error.message });
        }
    }

    async updateKarma() {
        try {
            const avgEfficiency = this.rankedResults.reduce((sum, r) => sum + r.efficiencyRatio, 0) / this.rankedResults.length;
            const ceimGain = avgEfficiency * 0.01;
            const payload = {
                identity_did: this.identityDid,
                karma_namespace: ARTEMIS_CONFIG.KARMA_NAMESPACE,
                ceim_gain: ceimGain,
                action_type: "corridor_synergy_ranking",
                timestamp_unix: Math.floor(Date.now() / 1000),
                result_count: this.rankedResults.length
            };
            const headers = this.getAuthHeaders();
            const response = await this.httpPost(`${this.apiBase}/karma/update`, payload, headers);
            this.emit("karmaUpdated", { currentKarma: response.currentKarma || 0 });
        } catch (error) {
            console.warn("Karma update failed:", error);
        }
    }

    async computeSha3Hash(data) {
        const encoder = new TextEncoder();
        const dataBuffer = encoder.encode(data);
        const hashBuffer = await crypto.subtle.digest("SHA-3-256", dataBuffer);
        const hashArray = Array.from(new Uint8Array(hashBuffer));
        return "0x" + hashArray.map(b => b.toString(16).padStart(2, "0")).join("");
    }

    async httpGet(url, headers = {}) {
        let retryCount = 0;
        while (retryCount < ARTEMIS_CONFIG.MAX_RETRY_COUNT) {
            try {
                const controller = new AbortController();
                const timeoutId = setTimeout(() => controller.abort(), ARTEMIS_CONFIG.REQUEST_TIMEOUT);
                const response = await fetch(url, {
                    method: "GET",
                    headers: { ...headers, "Content-Type": "application/json" },
                    signal: controller.signal
                });
                clearTimeout(timeoutId);
                if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                return await response.text();
            } catch (error) {
                retryCount++;
                if (retryCount >= ARTEMIS_CONFIG.MAX_RETRY_COUNT) throw error;
                await this.sleep(1000 * retryCount);
            }
        }
        throw new Error("Max retries exceeded");
    }

    async httpPost(url, payload, headers = {}) {
        let retryCount = 0;
        while (retryCount < ARTEMIS_CONFIG.MAX_RETRY_COUNT) {
            try {
                const controller = new AbortController();
                const timeoutId = setTimeout(() => controller.abort(), ARTEMIS_CONFIG.REQUEST_TIMEOUT);
                const response = await fetch(url, {
                    method: "POST",
                    headers: { ...headers, ...this.getAuthHeaders(), "Content-Type": "application/json" },
                    body: JSON.stringify(payload),
                    signal: controller.signal
                });
                clearTimeout(timeoutId);
                if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                return await response.json();
            } catch (error) {
                retryCount++;
                if (retryCount >= ARTEMIS_CONFIG.MAX_RETRY_COUNT) throw error;
                await this.sleep(1000 * retryCount);
            }
        }
        throw new Error("Max retries exceeded");
    }

    getAuthHeaders() {
        const headers = {};
        if (this.karmaToken) headers["Authorization"] = `Bearer ${this.karmaToken}`;
        headers["X-Identity-DID"] = this.identityDid;
        headers["X-Kernel-Hash"] = ARTEMIS_CONFIG.KERNEL_HASH;
        return headers;
    }

    sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    on(event, callback) {
        if (!this.eventListeners.has(event)) this.eventListeners.set(event, []);
        this.eventListeners.get(event).push(callback);
    }

    off(event, callback) {
        if (!this.eventListeners.has(event)) return;
        const listeners = this.eventListeners.get(event);
        const index = listeners.indexOf(callback);
        if (index > -1) listeners.splice(index, 1);
    }

    emit(event, data) {
        if (!this.eventListeners.has(event)) return;
        this.eventListeners.get(event).forEach(callback => {
            try { callback(data); } catch (error) { console.error(`Event listener error (${event}):`, error); }
        });
    }

    render() {
        const container = document.getElementById(this.containerId);
        if (!container) return;
        container.innerHTML = `
            <div class="artemis-dashboard">
                <header class="dashboard-header">
                    <h1>Artemis Corridor Synergy Analyzer</h1>
                    <div class="kernel-status">
                        <span class="kernel-hash">${ARTEMIS_CONFIG.KERNEL_HASH.substring(0, 16)}...</span>
                        <span class="contract-version">v${ARTEMIS_CONFIG.CONTRACT_VERSION}</span>
                    </div>
                </header>
                <main class="dashboard-main">
                    <section class="corridor-section">
                        <h2>Phoenix Corridors</h2>
                        <div id="corridor-list" class="data-list"></div>
                        <div id="corridor-map" class="map-container"></div>
                    </section>
                    <section class="intervention-section">
                        <h2>Available Interventions</h2>
                        <div id="intervention-list" class="data-list"></div>
                    </section>
                    <section class="ranking-section">
                        <h2>Ranked Interventions</h2>
                        <button id="rank-btn" class="action-btn">Run Optimization</button>
                        <div id="results-table" class="results-container"></div>
                    </section>
                    <section class="ledger-section">
                        <h2>Ledger Status</h2>
                        <div id="ledger-status" class="status-indicator">Pending</div>
                    </section>
                </main>
                <footer class="dashboard-footer">
                    <span>Repository: <a href="${ARTEMIS_CONFIG.REPOSITORY}" target="_blank">${ARTEMIS_CONFIG.REPOSITORY}</a></span>
                    <span>Karma: <span id="karma-display">0.00</span></span>
                </footer>
            </div>
        `;
        document.getElementById("rank-btn").addEventListener("click", () => this.rankInterventions());
    }

    renderCorridorList() {
        const container = document.getElementById("corridor-list");
        if (!container) return;
        container.innerHTML = this.corridors.map(c => `
            <div class="list-item">
                <span class="item-id">${c.corridorId}</span>
                <span class="item-coords">${c.lat.toFixed(4)}, ${c.lon.toFixed(4)}</span>
                <span class="item-ker">K:${c.K.toFixed(2)} E:${c.E.toFixed(2)} R:${c.R.toFixed(2)}</span>
            </div>
        `).join("");
    }

    renderInterventionList() {
        const container = document.getElementById("intervention-list");
        if (!container) return;
        container.innerHTML = this.interventions.map(i => `
            <div class="list-item">
                <span class="item-id">${i.interventionId}</span>
                <span class="item-type">${i.interventionType}</span>
                <span class="item-cost">$${i.costUsd.toLocaleString()}</span>
            </div>
        `).join("");
    }

    renderResultsTable() {
        const container = document.getElementById("results-table");
        if (!container) return;
        if (this.rankedResults.length === 0) {
            container.innerHTML = "<p class="no-results">No results yet. Run optimization to see ranked interventions.</p>";
            return;
        }
        container.innerHTML = `
            <table class="results-table">
                <thead>
                    <tr>
                        <th>Corridor</th>
                        <th>Intervention</th>
                        <th>Efficiency</th>
                        <th>Cost (USD)</th>
                        <th>K</th>
                        <th>E</th>
                        <th>R</th>
                        <th>Safety</th>
                    </tr>
                </thead>
                <tbody>
                    ${this.rankedResults.map(r => `
                        <tr>
                            <td>${r.corridorId}</td>
                            <td>${r.interventionId}</td>
                            <td>${r.efficiencyRatio.toFixed(6)}</td>
                            <td>$${r.costUsd.toLocaleString()}</td>
                            <td>${r.projectedKer[0].toFixed(3)}</td>
                            <td>${r.projectedKer[1].toFixed(3)}</td>
                            <td>${r.projectedKer[2].toFixed(3)}</td>
                            <td class="safety-${r.safetyFlags === 0 ? "pass" : "fail"}">${r.safetyFlags === 0 ? "PASS" : "FAIL"}</td>
                        </tr>
                    `).join("")}
                </tbody>
            </table>
        `;
    }

    renderCorridorMap() {
        const container = document.getElementById("corridor-map");
        if (!container) return;
        container.innerHTML = `<div class="map-placeholder">Map visualization for ${this.corridors.length} Phoenix corridors</div>`;
    }

    async updateKernelStatus() {
        try {
            const response = await this.httpGet(`${this.apiBase}/config/current`);
            const statusEl = document.querySelector(".kernel-status");
            if (statusEl) {
                statusEl.classList.add("status-online");
            }
        } catch (error) {
            console.warn("Kernel status check failed:", error);
        }
    }

    setKarmaToken(token) {
        this.karmaToken = token;
        this.updateKarmaDisplay();
    }

    async updateKarmaDisplay() {
        try {
            const headers = this.getAuthHeaders();
            const response = await this.httpGet(`${this.apiBase}/karma/current`, headers);
            const displayEl = document.getElementById("karma-display");
            if (displayEl && response.currentKarma !== undefined) {
                displayEl.textContent = response.currentKarma.toFixed(2);
            }
        } catch (error) {
            console.warn("Karma display update failed:", error);
        }
    }

    destroy() {
        this.eventListeners.clear();
        this.corridors = [];
        this.interventions = [];
        this.rankedResults = [];
    }
}

if (typeof module !== "undefined" && module.exports) {
    module.exports = { ArtemisDashboard, ARTEMIS_CONFIG };
}
