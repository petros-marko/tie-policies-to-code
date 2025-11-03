TOKEN=$(curl --request POST \
  --url https://dev-rpx7rpje8hqht13l.us.auth0.com/oauth/token \
  --header 'content-type: application/json' \
  --data '{
    "grant_type":"password",
    "username":"alice@example.com",
    "password":"Secret123!",
    "scope":"openid profile email",
    "client_id":"yQZPcSUF0zZX2TwYwunDTOefH84FtPv5",
    "client_secret":"Ugj7s6A5qDB_DUBZ9dhKdalY564c_K0kDmy6zp1MjCM_ctEN3Mnn9nftng1Gdxtm",
    "audience":"http://localhost:3000/"
  }' | jq -r '.access_token')

curl --request POST http://localhost:3000/profile/690918c9c9cad97dea460d53 \
    --header "Content-Type: application/json" \
    --header "Authorization: Bearer $TOKEN" \
    --data '{"full_name":"alice","email":"alice@example.com"}'

curl --request GET http://localhost:3000/profile/690918c9c9cad97dea460d53 \
    --header "Authorization: Bearer $TOKEN"