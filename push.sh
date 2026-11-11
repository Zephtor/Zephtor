#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

if git ls-files --error-unmatch .env >/dev/null 2>&1; then
  echo "FEHLER: .env wird noch von Git verfolgt."
  echo "Entferne sie zuerst mit: git rm --cached .env"
  exit 1
fi

read -r -p "Commit-Name/Beschreibung: " message
if [[ -z "$message" ]]; then
  echo "Die Beschreibung darf nicht leer sein."
  exit 1
fi

git add -A

if ! git diff --cached --quiet; then
  git commit -m "$message"
else
  echo "Keine neuen Änderungen zum Committen."
fi

branch=$(git branch --show-current)
if [[ -z "$branch" ]]; then
  echo "FEHLER: Kein aktueller Git-Branch."
  exit 1
fi

git push -u origin "$branch"
