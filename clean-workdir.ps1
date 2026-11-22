#Requires -Version 5.1
<#
.SYNOPSIS
    Allège le dossier de travail de Navi (carte N-079).

.DESCRIPTION
    Supprime uniquement des éléments temporaires et régénérables :
      navi-test.pdf       fichier de test Edge PDF, jamais utilisé (docs/s1-16-compat.md)
      kanban.html.bak     sauvegarde locale du tableau, ignorée par git
                          (ATLAS la recrée à chaque écriture du tableau)
      dist/               sortie de build Vite
      worktrees/          dossier de travail, supprimé seulement s'il est vide
      src-tauri/target/   cache de compilation Cargo (uniquement avec -IncludeCargo)

    Ne supprime jamais un élément suivi par git : chaque cible est contrôlée
    avec git ls-files avant effacement. models/ et node_modules/ ne sont
    jamais touchés.

    Sans -Apply, le script ne fait qu'afficher ce qu'il supprimerait.

.PARAMETER Apply
    Efface réellement les cibles. Sans ce commutateur : simulation.

.PARAMETER IncludeCargo
    Ajoute src-tauri/target (cache de compilation). La prochaine compilation
    repart de zéro : plusieurs minutes, et cargo test reconstruit tout.

.EXAMPLE
    .\clean-workdir.ps1
    Simulation : liste les cibles et les tailles.

.EXAMPLE
    .\clean-workdir.ps1 -Apply
    Efface les éléments légers.

.EXAMPLE
    .\clean-workdir.ps1 -Apply -IncludeCargo
    Efface aussi le cache Cargo (11,8 Go sur la machine de référence).
#>
[CmdletBinding()]
param(
    [switch]$Apply,
    [switch]$IncludeCargo
)

$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot

# Affichage correct des accents dans la console Windows.
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

function Get-TailleMo {
    param([string]$Chemin)
    if (-not (Test-Path -LiteralPath $Chemin)) { return 0 }
    $item = Get-Item -LiteralPath $Chemin -Force
    if ($item.PSIsContainer) {
        $somme = (Get-ChildItem -LiteralPath $Chemin -Recurse -Force -File -ErrorAction SilentlyContinue |
            Measure-Object -Property Length -Sum).Sum
    }
    else {
        $somme = $item.Length
    }
    if ($null -eq $somme) { return 0 }
    return [math]::Round($somme / 1MB, 1)
}

function Test-SuiviParGit {
    param([string]$Relatif)
    Push-Location -LiteralPath $root
    try {
        $suivi = & git ls-files -- $Relatif 2>$null
        if ($LASTEXITCODE -ne 0) { return $false }
        return [bool]$suivi
    }
    finally { Pop-Location }
}

function Format-Taille {
    param([double]$Mo)
    if ($Mo -ge 1024) { return '{0:N2} Go' -f ($Mo / 1024) }
    return '{0:N1} Mo' -f $Mo
}

$cibles = @(
    [pscustomobject]@{ Chemin = 'navi-test.pdf'; Genre = 'fichier'; Note = 'test Edge PDF inutilisé (s1-16)'; Cargo = $false }
    [pscustomobject]@{ Chemin = 'kanban.html.bak'; Genre = 'fichier'; Note = 'sauvegarde du tableau (recréée par ATLAS)'; Cargo = $false }
    [pscustomobject]@{ Chemin = 'dist'; Genre = 'dossier'; Note = 'sortie de build Vite'; Cargo = $false }
    [pscustomobject]@{ Chemin = 'worktrees'; Genre = 'dossier-vide'; Note = 'dossier de travail vide'; Cargo = $false }
    [pscustomobject]@{ Chemin = 'src-tauri/target'; Genre = 'dossier'; Note = 'cache de compilation Cargo'; Cargo = $true }
)

Write-Host ''
Write-Host 'Navi - allègement du dossier de travail (N-079)'
Write-Host "Racine : $root"
if ($Apply) {
    $mode = 'application'
    if ($IncludeCargo) { $mode += ' + cache Cargo' }
}
else {
    $mode = 'simulation (ajouter -Apply pour effacer)'
}
Write-Host "Mode   : $mode"
Write-Host ''

$aSupprimer = @()
$ignore = @()

foreach ($cible in $cibles) {
    if ($cible.Cargo -and -not $IncludeCargo) { continue }

    $complet = Join-Path $root $cible.Chemin
    if (-not (Test-Path -LiteralPath $complet)) {
        $ignore += [pscustomobject]@{ Cible = $cible.Chemin; Raison = 'absent' }
        continue
    }

    if (Test-SuiviParGit -Relatif $cible.Chemin) {
        $ignore += [pscustomobject]@{ Cible = $cible.Chemin; Raison = 'suivi par git - refus' }
        continue
    }

    if ($cible.Genre -eq 'dossier-vide') {
        $contenu = Get-ChildItem -LiteralPath $complet -Force -ErrorAction SilentlyContinue
        if ($contenu) {
            $ignore += [pscustomobject]@{ Cible = $cible.Chemin; Raison = 'non vide - conserve' }
            continue
        }
    }

    $aSupprimer += [pscustomobject]@{
        Cible  = $cible.Chemin
        Taille = Get-TailleMo -Chemin $complet
        Note   = $cible.Note
        Complet = $complet
    }
}

$total = 0
foreach ($element in $aSupprimer) {
    $total += $element.Taille
    $etiquette = if ($element.Cible -eq 'src-tauri/target') { 'cargo' } else { 'léger' }
    Write-Host ('  [{0}] {1,-22} {2,10}  {3}' -f $etiquette, $element.Cible, (Format-Taille $element.Taille), $element.Note)
}

foreach ($element in $ignore) {
    Write-Host ('  [ignoré] {0,-22} {1,10}  {2}' -f $element.Cible, '', $element.Raison) -ForegroundColor DarkGray
}

Write-Host ''
Write-Host ("Total concerné : {0}" -f (Format-Taille $total))

if (-not $Apply) {
    Write-Host 'Simulation : rien n''a été supprimé.'
    return
}

if ($aSupprimer.Count -eq 0) {
    Write-Host 'Aucune cible à supprimer.'
    return
}

$avant = Get-TailleMo -Chemin $root
$echecs = 0

foreach ($element in $aSupprimer) {
    try {
        Remove-Item -LiteralPath $element.Complet -Recurse -Force -ErrorAction Stop
        Write-Host ("  supprimé : {0}" -f $element.Cible)
    }
    catch {
        $echecs++
        Write-Warning ("  échec sur {0} : {1}" -f $element.Cible, $_.Exception.Message)
    }
}

$apres = Get-TailleMo -Chemin $root
Write-Host ''
Write-Host ("Dossier de travail : {0} -> {1} ({2} libérés)" -f (Format-Taille $avant), (Format-Taille $apres), (Format-Taille ($avant - $apres)))

Push-Location -LiteralPath $root
try {
    $modifies = & git status --porcelain --untracked-files=no 2>$null
}
finally { Pop-Location }

if ($modifies) {
    Write-Host ''
    Write-Host 'Fichiers suivis modifiés (à vérifier) :' -ForegroundColor Yellow
    $modifies | ForEach-Object { Write-Host "  $_" }
}
else {
    Write-Host 'Aucun fichier suivi modifié par le nettoyage.'
}

if ($echecs -gt 0) { exit 1 }
