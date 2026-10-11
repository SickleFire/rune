$ErrorActionPreference = 'Stop'
$files = Get-ChildItem -Path 'src' -Filter '*.rs' -Recurse
foreach ($f in $files) {
    $t = [IO.File]::ReadAllText($f.FullName)
    $n = [regex]::Replace($t, '(?<![A-Za-z0-9_])rune::', 'rune_code::')
    $n = [regex]::Replace($n, '\bRuneHelper\b', 'RuneCodeHelper')
    if ($n -ne $t) {
        [IO.File]::WriteAllText($f.FullName, $n)
        $count = ([regex]::Matches($t, '(?<![A-Za-z0-9_])rune::|\bRuneHelper\b')).Count
        Write-Output ("updated " + $f.FullName + " (" + $count + " replacements)")
    }
}
