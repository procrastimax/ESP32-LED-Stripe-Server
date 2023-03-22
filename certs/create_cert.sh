#!/bin/bash
set -e

echo "---------------------"
echo "Creating Certificate"
echo "---------------------"

# delete previous files
rm -vf -- *.txt

# create a server private key
openssl genrsa -out server_priv.key 2048

# create a certificate for the ESP
openssl req -new -key server_priv.key -out server.csr

# have CA sign the certificate
openssl x509 -req -in server.csr -CA CA.pem.crt -CAkey CA_priv.key -CAcreateserial -out server.crt -days 3650 -sha256 -extfile cert.ext

# verify
openssl verify -CAfile CA.pem.crt server.crt

# convert private key and certificate into DER format
openssl rsa -in server_priv.key -outform DER -out server_priv.key.DER
openssl x509 -in server.crt -outform DER -out server.crt.DER

# create hexdump of server certificate
xxd -i server.crt.DER >> ./server_cert.txt

# create hexdump of server private key
xxd -i server_priv.key.DER >> ./server_key.txt

# delete unused files
rm -v -- *.csr
rm -v server.crt
rm -v server_priv.key

echo "---------------------"
echo "Certificate created"
echo "---------------------"
