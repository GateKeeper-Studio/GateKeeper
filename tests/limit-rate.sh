for i in $(seq 1 12); do
  echo -n "Request $i: "
  curl -s -o /dev/null -w "%{http_code}" -X POST http://localhost:8080/v1/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email":"user@example.com","password":"wrong","applicationId":"00000000-0000-0000-0000-000000000000","codeChallenge":"x","codeChallengeMethod":"S256","redirectUri":"http://localhost","responseType":"code","state":"s"}'
  echo
done