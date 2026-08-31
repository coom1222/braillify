param(
    [string]$ArchivePath = (Join-Path $PSScriptRoot '..\NIKL_Korean-Korean_Braille_Parallel_Corpus_2025_v1.0.zip'),
    [string]$OutputPath = (Join-Path $PSScriptRoot '..\test_cases\corpus\sentence.json')
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.IO.Compression.FileSystem

$archive = [System.IO.Compression.ZipFile]::OpenRead((Resolve-Path $ArchivePath))
$cases = [System.Collections.Generic.List[object]]::new()

try {
    foreach ($entry in $archive.Entries | Where-Object Name -like '*.json') {
        $reader = [System.IO.StreamReader]::new($entry.Open(), [System.Text.Encoding]::UTF8)
        try {
            $document = $reader.ReadToEnd() | ConvertFrom-Json
        }
        finally {
            $reader.Dispose()
        }

        foreach ($record in $document.parallel) {
            # NIKL uses U+0020 between cells. braillify's Unicode API represents
            # the same blank cell as U+2800, so retain both forms explicitly.
            $target = [string]$record.target
            $cases.Add([ordered]@{
                id = [string]$record.id
                original_id = [string]$record.original_id
                source_file = $entry.Name
                input = [string]$record.source
                target = $target
                unicode = $target.Replace(' ', [string][char]0x2800)
            })
        }
    }
}
finally {
    $archive.Dispose()
}

$outputDirectory = Split-Path -Parent $OutputPath
[System.IO.Directory]::CreateDirectory($outputDirectory) | Out-Null

[System.IO.File]::WriteAllText(
    $OutputPath,
    ($cases | ConvertTo-Json -Depth 3),
    [System.Text.UTF8Encoding]::new($false)
)

Write-Host "Imported $($cases.Count) NIKL parallel corpus records into $OutputPath"
