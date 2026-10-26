#!/usr/bin/env bash
# Sync profile-local skills to global skills directory
# Run periodically to promote earned skills across all profiles

HERMES_HOME="${HERMES_HOME:-$HOME/.hermes}"
GLOBAL_SKILLS="$HERMES_HOME/skills"
PROFILES_DIR="$HERMES_HOME/profiles"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

log "Starting skill sync from profile-local to global"

# Find all profiles with local skills
for profile_dir in "$PROFILES_DIR"/*/; do
    [ -d "$profile_dir/skills" ] || continue
    
    profile_name=$(basename "$profile_dir")
    log "Checking profile: $profile_name"
    
    # Compare each local skill against global
    for local_skill in "$profile_dir/skills"/*; do
        [ -f "$local_skill" ] || continue
        
        skill_name=$(basename "$local_skill")
        global_skill="$GLOBAL_SKILLS/$skill_name"
        
        if [ ! -f "$global_skill" ]; then
            # New skill not in global - promote it
            log "  Promoting new skill: $skill_name"
            cp "$local_skill" "$global_skill"
        elif [ "$local_skill" -nt "$global_skill" ]; then
            # Local skill is newer than global - update global
            log "  Updating global skill: $skill_name"
            cp "$local_skill" "$global_skill"
        fi
    done
done

log "Skill sync complete"
