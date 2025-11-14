#!/bin/bash

# ============================================================================
# Messaging App CLI Client
# ============================================================================
# A command-line client for interacting with your messaging app
# ============================================================================

# =========================
# CONFIGURATION
# =========================
AUTH0_DOMAIN="dev-rpx7rpje8hqht13l.us.auth0.com"
CLIENT_ID="yQZPcSUF0zZX2TwYwunDTOefH84FtPv5"
CLIENT_SECRET="Ugj7s6A5qDB_DUBZ9dhKdalY564c_K0kDmy6zp1MjCM_ctEN3Mnn9nftng1Gdxtm"
API_AUDIENCE="http://localhost:3000/"
API_URL="http://localhost:3000"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# =========================
# HELPER FUNCTIONS
# =========================

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${CYAN}→${NC} $1"
}

pretty_json() {
    if command -v jq &> /dev/null; then
        echo "$1" | jq -C 2>/dev/null || echo "$1"
    else
        echo "$1"
    fi
}

api_call() {
    local method=$1
    local endpoint=$2
    local data=$3
    
    if [ -z "$data" ]; then
        RESPONSE=$(curl --silent \
          --write-out "\nHTTP_STATUS:%{http_code}" \
          -X "${method}" \
          "${API_URL}${endpoint}" \
          -H "Authorization: Bearer ${TOKEN}")
    else
        RESPONSE=$(curl --silent \
          --write-out "\nHTTP_STATUS:%{http_code}" \
          -X "${method}" \
          "${API_URL}${endpoint}" \
          -H "Authorization: Bearer ${TOKEN}" \
          -H "Content-Type: application/json" \
          -d "$data")
    fi
    
    HTTP_STATUS=$(echo "$RESPONSE" | grep "HTTP_STATUS" | cut -d: -f2)
    BODY=$(echo "$RESPONSE" | sed '$d')
    
    if [[ "$HTTP_STATUS" =~ ^2 ]]; then
        return 0
    else
        return 1
    fi
}

authenticate() {
    if [ -n "$TOKEN" ]; then
        return 0
    fi
    
    # Prompt for username and password if not set
    if [ -z "$USERNAME" ]; then
        read -p "Username (email): " USERNAME
    fi
    
    if [ -z "$PASSWORD" ]; then
        read -p "Password: " PASSWORD
    fi

    TOKEN=$(curl --silent --request POST \
      --url https://${AUTH0_DOMAIN}/oauth/token \
      --header 'content-type: application/json' \
      --data "{
        \"grant_type\": \"password\",
        \"username\": \"${USERNAME}\",
        \"password\": \"${PASSWORD}\",
        \"scope\": \"openid profile email\",
        \"client_id\": \"${CLIENT_ID}\",
        \"client_secret\": \"${CLIENT_SECRET}\",
        \"audience\": \"${API_AUDIENCE}\"
      }" | jq -r '.access_token')
    
    if [ -z "$TOKEN" ]; then
        print_error "Authentication failed - check your username and password"
        exit 1
    fi
    
    user_id=$(echo $TOKEN | cut -d'.' -f2 | base64 -d 2>/dev/null | grep -o '"sub":"[^"]*' | cut -d'"' -f4 | cut -d'|' -f2)

    print_success "Authenticated as $USERNAME with id $user_id"
}

# =========================
# COMMANDS
# =========================

cmd_profile_create() {
    authenticate
    
    echo ""
    echo "Create Profile"
    echo "-------------"
    read -p "full name: " full_name
    read -p "email: " email
    data=$(jq -n \
      --arg full_name "$full_name" \
      --arg email "$email" \
      '{full_name: $full_name, email: $email}')
    
    if api_call "POST" "/profile/${user_id}" "$data"; then
        print_success "Profile created"
        pretty_json "$BODY"
    else
        print_error "Failed to create profile (HTTP $HTTP_STATUS)"
        echo "$BODY"
    fi
}

cmd_profile_update() {
    authenticate
    
    echo ""
    echo "Update Profile"
    echo "-------------"
    read -p "full name: " full_name
    read -p "email: " email
    data=$(jq -n \
      --arg full_name "$full_name" \
      --arg email "$email" \
      '{full_name: $full_name, email: $email}')
    
    if api_call "PUT" "/profile/${user_id}" "$data"; then
        print_success "Profile updated"
        pretty_json "$BODY"
    else
        print_error "Failed to update profile (HTTP $HTTP_STATUS)"
        echo "$BODY"
    fi
}

cmd_profile_get() {
    authenticate
    
    user_id="$1"
    
    if [ -z "$user_id" ]; then
        read -p "user id: " user_id
    fi
    
    print_info "Fetching profile..."
    
    if api_call "GET" "/profile/${user_id}"; then
        pretty_json "$BODY"
    else
        print_error "Failed to get profile (HTTP $HTTP_STATUS)"
        echo "$BODY"
    fi
}

cmd_friend_add() {
    authenticate
    
    user_id="$1"
    
    if [ -z "$user_id" ]; then
        read -p "friend's user id: " user_id
    fi
    
    print_info "Sending friend request to $user_id..."
    
    if api_call "POST" "/friendship/${user_id}"; then
        print_success "Friend request sent"
        pretty_json "$BODY"
    else
        print_error "Failed to send friend request (HTTP $HTTP_STATUS)"
        echo "$BODY"
    fi
}

cmd_friend_accept() {
    authenticate
    
    user_id="$1"
    
    if [ -z "$user_id" ]; then
        read -p "friend's user id: " user_id
    fi
    
    print_info "Accepting friend request from $user_id..."
    
    if api_call "POST" "/friendship/${user_id}/accept"; then
        print_success "Friend request accepted"
        pretty_json "$BODY"
    else
        print_error "Failed to accept friend request (HTTP $HTTP_STATUS)"
        echo "$BODY"
    fi
}

cmd_friend_remove() {
    authenticate
    
    user_id="$1"
    
    if [ -z "$user_id" ]; then
        read -p "friend's user id: " user_id
    fi
    
    print_info "Removing friend $user_id..."
    
    if api_call "DELETE" "/friendship/${user_id}"; then
        print_success "Friend removed"
        pretty_json "$BODY"
    else
        print_error "Failed to remove friend (HTTP $HTTP_STATUS)"
        echo "$BODY"
    fi
}

cmd_message_send() {
    authenticate
    
    user_id="$1"
    shift
    text="$@"
    
    if [ -z "$user_id" ]; then
        read -p "Recipient user ID: " user_id
    fi
    
    if [ -z "$text" ]; then
        read -p "Message: " text
    fi
    
    data=$(jq -n \
      --arg text "$text" \
      '{text: $text}')
    
    print_info "Sending message..."
    
    if api_call "POST" "/conversation_with/${user_id}" "$data"; then
        print_success "Message sent"
        pretty_json "$BODY"
    else
        print_error "Failed to send message (HTTP $HTTP_STATUS)"
        echo "$BODY"
    fi
}

cmd_conversation_get() {
    authenticate
    
    user_id="$1"
    
    if [ -z "$user_id" ]; then
        read -p "User ID: " user_id
    fi
    
    print_info "Fetching conversation with $user_id..."
    
    if api_call "GET" "/conversation_with/${user_id}"; then
        pretty_json "$BODY"
    else
        print_error "Failed to get conversation (HTTP $HTTP_STATUS)"
        echo "$BODY"
    fi
}

cmd_message_latest() {
    authenticate
    
    user_id="$1"
    
    if [ -z "$user_id" ]; then
        read -p "User ID: " user_id
    fi
    
    print_info "Fetching latest message with $user_id..."
    
    if api_call "GET" "/conversation_with/${user_id}/last"; then
        pretty_json "$BODY"
    else
        print_error "Failed to get latest message (HTTP $HTTP_STATUS)"
        echo "$BODY"
    fi
}

show_help() {
    cat << EOF
Messaging App CLI Client

Usage: $0 <command> [arguments]

Commands:
  Profile Management:
    profile create              Create a new profile (interactive)
    profile update              Update your profile (interactive)
    profile get <user_id>       Get a user's profile

  Friends:
    friend add <user_id>        Send a friend request
    friend accept <user_id>     Accept a friend request
    friend remove <user_id>     Remove a friend

  Messaging:
    msg send <user_id> [text]   Send a message to a user
    msg latest <user_id>        Get latest message with a user
    msg conversation <user_id>      Get conversation history with a user

  General:
    help                        Show this help message

Examples:
  $0 profile get user123
  $0 friend add user456
  $0 msg send user456 "Hello there!"
  $0 conversation user456

Environment Variables:
  AUTH0_DOMAIN                 Your Auth0 domain
  CLIENT_ID                    Your Auth0 client ID
  CLIENT_SECRET                Your Auth0 client secret
  API_AUDIENCE                 Your API audience/identifier
  API_URL                      API base URL (default: http://localhost:3000)
EOF
}

# =========================
# MAIN
# =========================

COMMAND="$1"
shift

case "$COMMAND" in
    profile)
        SUB_COMMAND="$1"
        shift
        case "$SUB_COMMAND" in
            create) cmd_profile_create ;;
            update) cmd_profile_update ;;
            get) cmd_profile_get "$@" ;;
            *) show_help ;;
        esac
        ;;
    friend)
        SUB_COMMAND="$1"
        shift
        case "$SUB_COMMAND" in
            add) cmd_friend_add "$@" ;;
            accept) cmd_friend_accept "$@" ;;
            remove) cmd_friend_remove "$@" ;;
            *) show_help ;;
        esac
        ;;
    msg)
        SUB_COMMAND="$1"
        shift
        case "$SUB_COMMAND" in
            send) cmd_message_send "$@" ;;
            latest) cmd_message_latest "$@" ;;
            conversation) cmd_conversation_get "$@" ;;
            *) show_help ;;
        esac
        ;;
    *) 
        print_error "Unknown command: $COMMAND"
        echo ""
        show_help
        exit 1
        ;;
esac