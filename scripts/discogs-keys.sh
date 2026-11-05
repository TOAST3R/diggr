# Sourced by the release scripts: the consumer key and secret Diggr is registered with on
# Discogs, built into release builds for Connect with Discogs (`option_env!` in
# crates/dig/src/discogs/oauth.rs). Never committed: they come from the environment if set,
# otherwise from the maintainer's macOS Keychain (`diggr-discogs-key`, `diggr-discogs-secret`).
#
# After sourcing, DISCOGS_CONSUMER_KEY and DISCOGS_CONSUMER_SECRET are exported when both were
# found, and DISCOGS_KEYS is 1 (else 0, and neither is set: the build has no Connect with
# Discogs, only the personal token). `discogs_keys_say` prints which, without the values.

discogs_keychain() {
    command -v security >/dev/null 2>&1 || return 0
    security find-generic-password -s "$1" -w 2>/dev/null || true
}

: "${DISCOGS_CONSUMER_KEY:=$(discogs_keychain diggr-discogs-key)}"
: "${DISCOGS_CONSUMER_SECRET:=$(discogs_keychain diggr-discogs-secret)}"
if [[ -n "$DISCOGS_CONSUMER_KEY" && -n "$DISCOGS_CONSUMER_SECRET" ]]; then
    DISCOGS_KEYS=1
    export DISCOGS_CONSUMER_KEY DISCOGS_CONSUMER_SECRET
else
    DISCOGS_KEYS=0
    unset DISCOGS_CONSUMER_KEY DISCOGS_CONSUMER_SECRET
fi

discogs_keys_say() {
    if ((DISCOGS_KEYS)); then
        echo "Discogs keys found: Connect with Discogs is built in."
    else
        echo "No Discogs keys (Keychain diggr-discogs-key and diggr-discogs-secret, or" \
            "DISCOGS_CONSUMER_KEY and DISCOGS_CONSUMER_SECRET): this build has no Connect with" \
            "Discogs, only the personal token."
    fi
}
