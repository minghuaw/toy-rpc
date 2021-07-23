# TLS Support

Support for TLS is implemented with `rustls` and its `async` derivatives `async-rustls` and `tokio-rustls`. 

An example using a self-signed certificate can be found in the [GitHub repo](https://github.com/minghuaw/toy-rpc/tree/main/examples/tokio_tls).

More detailed documentations are underway.

## [How to generate self-signed CA and certs](https://itnext.io/practical-guide-to-securing-grpc-connections-with-go-and-tls-part-1-f63058e9d6d1) (an example)

1. Create Root signing key: `openssl genrsa -out ca.key 4096`
2. Generate self-signed Root certificate: `openssl req -new -x509 -key ca.key -sha256 -subj "/C=CA/ST=BC/L=Vancouver/O=Example, Inc." -days 365 -out ca.cert`
3. Create a key for server: `openssl genrsa -out service.key 4096`
4. Create a signing request (CSR): `openssl req -new -key service.key -out service.csr -config certificate.conf` (see section below for details of `certificate.conf`)
5. Generate a certificate for the server: `openssl x509 -req -in service.csr -CA ca.cert -CAkey ca.key -CAcreateserial -out service.pem -days 365 -sha256 -extfile certificate.conf -extensions req_ext`

`certificate.conf`

```bash
[req]
default_bits = 4096
prompt = no
default_md = sha256
req_extensions = req_ext
distinguished_name = dn
[dn]
C = CA
ST = BC
L = Vancouver
O = Example, Inc.
CN = localhost
[req_ext]
subjectAltName = @alt_names
[alt_names]
DNS.1 = localhost
IP.1 = ::1
IP.2 = 127.0.0.1
```