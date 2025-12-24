#!/bin/bash
# INTENTIONAL VULNERABILITIES - DO NOT USE IN PRODUCTION
# Test fixture for security scanner validation

# BASH-001: Unquoted variable - command injection
user_input="$1"
echo $user_input | grep pattern  # BAD: Unquoted variable
eval $command_from_env           # BAD: Eval with variable

# BASH-002: Insecure temp file creation
TMPFILE=/tmp/myapp.tmp          # BAD: Predictable name
touch /tmp/lockfile.lock        # BAD: Not using mktemp
echo "data" >/tmp/$$            # BAD: $$ is predictable

# BASH-003: Curl without certificate verification
curl -k https://api.example.com/data      # BAD: Insecure flag
curl --insecure https://internal.api/     # BAD: Insecure flag
wget --no-check-certificate https://dl.example.com/pkg  # BAD: No cert check

# BASH-004: Dangerous file permissions
chmod 777 /var/www/app          # BAD: World writable
chmod 666 config.txt            # BAD: World writable
chmod -R 777 /opt/app           # BAD: Recursive world writable
chmod o+w secrets.txt           # BAD: Others can write
umask 000                       # BAD: No default protection

# BASH-005: Eval with external input
user_cmd="$2"
eval "$user_cmd"                # BAD: Eval with user input
eval ${OPTIONS}                 # BAD: Eval with variable
source $CONFIG_FILE             # BAD: Sourcing untrusted path
. $PLUGIN_PATH                  # BAD: Dot-sourcing variable

# SAFE PATTERNS (should not trigger)
safe_var="value"
echo "${safe_var}"              # GOOD: Quoted variable
SAFE_TEMP=$(mktemp)             # GOOD: Using mktemp
curl https://api.example.com/   # GOOD: No insecure flag
chmod 600 private.key           # GOOD: Restrictive permissions
chmod 755 script.sh             # GOOD: Standard executable
