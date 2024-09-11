# Rocks Works

Lightweight proxy, initially implemented for Vless protocol

## Building the Docker Image

To build the Docker image, run the following command in the root directory of the project:

```sh
docker build -t rocks_works .
```

This command will create a Docker image named `rocks_works` using the `Dockerfile` provided in the project.

## Running the Docker Container

To run the Docker container, use the following command:

```sh
docker run -p 34434:34434 rocks_works
```

This command will start a container from the `rocks_works` image and map port `34434` of the host to port `34434` of the container.

## Example `v2ray` config

```json
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": 1081,
      "listen": "127.0.0.1",
      "protocol": "socks",
      "settings": {
        "auth": "noauth",
        "udp": false,
        "ip": "127.0.0.1"
      }
    }
  ],
  "outbounds": [
    {
      "protocol": "vless",
      "settings": {
        "vnext": [
          {
            "address": "127.0.0.1",
            "port": 34434,
            "users": [
              {
                "id": "74657374-0000-0000-0000-000000000000",
                "encryption": "none"
              }
            ]
          }
        ]
      },
      "tag": "rocks"
    }
  ]
}
```
