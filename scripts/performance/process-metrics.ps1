param([Parameter(Mandatory = $true)][int]$TargetProcessId)

$ErrorActionPreference = 'Stop'
$targetProcess = [System.Diagnostics.Process]::GetProcessById($TargetProcessId)

try {
    while ($null -ne ($request = [Console]::ReadLine())) {
        if ($request -eq 'stop') {
            break
        }
        if ($request -ne 'sample') {
            throw 'Unknown measurement request.'
        }
        $targetProcess.Refresh()
        if ($targetProcess.HasExited) {
            throw 'The measured process has exited.'
        }
        $sample = [ordered]@{
            cpuMs = $targetProcess.TotalProcessorTime.TotalMilliseconds
            residentBytes = $targetProcess.WorkingSet64
            peakResidentBytes = $targetProcess.PeakWorkingSet64
            privateCommittedBytes = $targetProcess.PrivateMemorySize64
        }
        [Console]::WriteLine(($sample | ConvertTo-Json -Compress))
    }
} finally {
    $targetProcess.Dispose()
}
