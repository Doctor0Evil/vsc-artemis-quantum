param(
    [ValidateSet('lint','build','ecosafety','all')]
    [string]$Mode = 'all',
    [string]$SpecRepoPath = '../ecosafety-spec'
)

$ErrorActionPreference = 'Stop'

function Invoke-RustBuild {
    Write-Host "[ecosafety] Building Rust workspace..."
    cargo build --workspace --locked
}

function Invoke-ALNValidate {
    param([string]$SpecPath)
    Write-Host "[ecosafety] Validating ALN schemas and contracts..."
    & "$SpecPath/tools/alnci" validate --schema "$SpecPath/aln/schema/NodeShard_v1.aln" --contract "$SpecPath/aln/contracts/EcoSafetyKernel_v1.aln" --contract "$SpecPath/aln/contracts/QuantumOrthogonalityGuard.aln"
}

function Invoke-RepoContractsValidate {
    param([string]$SpecPath)
    Write-Host "[ecosafety] Validating repo-specific ALN bindings..."
    Get-ChildItem -Path "aln/contracts/*ControllerSpec.aln" -ErrorAction SilentlyContinue | ForEach-Object {
        & "$SpecPath/tools/alnci" validate --contract $_.FullName
    }
}

function Invoke-EcoSafetyKernelTests {
    Write-Host "[ecosafety] Running ecosafety kernel tests..."
    if (Test-Path "Cargo.toml") { cargo test --tests ecosafety -- --nocapture }
}

function Invoke-EcoSafetySummary {
    Write-Host "[ecosafety] Computing KER summary..."
    if (Test-Path "target/debug/ker_summary") {
        $json = & "target/debug/ker_summary"
        $obj = $json | ConvertFrom-Json
        Write-Host ("[ecosafety] K={0}, E={1}, R={2}, status={3}" -f $obj.K, $obj.E, $obj.R, $obj.status)
        if ($obj.status -ne 'ok') { throw "Ecosafety status is '$($obj.status)', failing." }
    }
}

switch ($Mode) {
    'lint' { Invoke-ALNValidate -SpecPath $SpecRepoPath; Invoke-RepoContractsValidate -SpecPath $SpecRepoPath }
    'build' { if (Test-Path "Cargo.toml") { Invoke-RustBuild } }
    'ecosafety' { Invoke-ALNValidate -SpecPath $SpecRepoPath; Invoke-RepoContractsValidate -SpecPath $SpecRepoPath; Invoke-EcoSafetyKernelTests; Invoke-EcoSafetySummary }
    'all' { if (Test-Path "Cargo.toml") { Invoke-RustBuild }; Invoke-ALNValidate -SpecPath $SpecRepoPath; Invoke-RepoContractsValidate -SpecPath $SpecRepoPath; Invoke-EcoSafetyKernelTests; Invoke-EcoSafetySummary }
}
