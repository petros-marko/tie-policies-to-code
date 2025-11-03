TOKEN=$(curl --request POST \
  --url https://dev-rpx7rpje8hqht13l.us.auth0.com/oauth/token \
  --header 'content-type: application/json' \
  --data '{
    "client_id":"XKfU7D7IaMwVW4LgxjfmZ5yULgSKAbk3",
    "client_secret":"oOtUHOh7n0gL3xUxQv3R2uWqwMI8qkhPzQ02goQvRGsQyA7K0qf4k29ViM8CLy8S",
    "audience":"https://dev-rpx7rpje8hqht13l.us.auth0.com/api/v2/",
    "grant_type":"client_credentials"
  }' | jq -r '.access_token')

curl -L -g 'dev-rpx7rpje8hqht13l.us.auth0.com/api/v2/users' \
  --header "Content-Type: application/json" \
  --header "Authorization: Bearer $TOKEN" \
  --data '{
    "connection": "Username-Password-Authentication",
    "email": "alice@example.com",
    "username": "alice",
    "password": "Secret123!",
    "email_verified": true,
    "user_metadata": {"role":"tester"}
  }'