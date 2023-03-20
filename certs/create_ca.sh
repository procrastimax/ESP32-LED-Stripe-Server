#!/usr/bin/env bash

# Creates a new CA (CERTIFICATE AUTHORITY) certificate
# Also creates a CA.der.crt which can be used to be imported by android

echo "Creating a new certificate for a certificate authority, to sign new certificates later"
read -p "Are you sure you want to continue? [y/n] " DEC

if [ "$DEC" = "y" ]; then
    #echo 'basicConstraints=CA:true' > android_options.txt
    openssl genrsa -out CA_priv.key 2048
    openssl req -x509 -new -nodes -key CA_priv.key -sha256 -days 3650 -out CA.pem.crt
    openssl x509 -inform PEM -outform DER -in CA.pem.crt -out CA.der.crt
else
    echo "Quitting..."
fi
