openssl req -newkey rsa:4096 -keyout backend.key -nodes -out backend.req -days 365 -subj '/O=si/CN=test-backend/emailAddress=mdp@si-int.eu' -addext 'basicConstraints=CA:FALSE'
#openssl x509 -req -in backend.req -CA ca.crt -CAkey ca.key -CAcreateserial -out backend.crt
openssl x509 -req -in backend.req -CA ca.crt -CAkey ca.key -out backend.crt
openssl x509 -in backend.crt -text
