#!/bin/sh

case "$1" in
    help|"" )
        echo "Usage: $0 <subcommand>"
        echo "Subcommands:"
        echo "  help      Show this help"
        echo "  rpull     Poll git pull until remote changes are fetched"
        echo "  cbranch   Checkout most recent codex/* branch"
        ;;
    rpull)
        i=0
        while :; do
            i=$((i+1))
            out=$(git pull 2>&1)
            if echo "$out" | grep -q "Already up to date."; then
                echo "fetching git ($i)"
            else
                echo "$out"
                break
            fi
        done
        ;;
    cbranch)
        git fetch
        branch=$(git branch -r --sort=-committerdate | grep 'origin/codex/' | head -n1 | sed 's#origin/##')
        if [ -n "$branch" ]; then
            git checkout "$branch"
        else
            echo "No codex branch found"
        fi
        ;;
    *)
        echo "Unknown subcommand: $1"
        exit 1
        ;;
esac
