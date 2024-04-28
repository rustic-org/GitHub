# GitHub

This is a simple API backed GH actions project, that backs up any files that were updated in the default branch.

### Environment Variables

**Mandatory**
- **authorization** - Token stored in GitHub actions.
- **github_source** - Directory to store the backup.

**Optional**
- **debug**: Boolean flag to enable debug level logging. Defaults to `false`
- **utc_logging**: Boolean flag to set timezone to UTC in the output logs. Defaults to `true`
- **server_host**: IP address to host the server. Defaults to `127.0.0.1` / `localhost`
- **server_port**: Port number to host the application. Defaults to `8000`
- **workers**: Number of workers to spin up for the server. Defaults to the number of physical cores.
- **max_connections**: Maximum number of concurrent connections per worker. Defaults to `3`
- **max_payload_size**: Maximum size of files that can be uploaded from the UI. Defaults to `100 MB`
  > Input should be in the format, `10 MB`, `3 GB` - _inputs are case insensitive_
- **websites**: Vector of websites (_supports regex_) to add to CORS configuration. _Required only if tunneled via CDN_
- **key_file**: Path to the private key file for SSL certificate. Defaults to `None`
- **cert_file**: Path to the full chain file for SSL certificate. Defaults to `None`

### Steps
- The API should be running independently.
- The GH actions, will send the changes to the API which will be stored in the backup location.

### Docker

**Build**
```shell
docker build -t github .
```

**Run**
```shell
docker run github
```

**Copy executable**
```shell
docker cp $(docker ps -aqf "ancestor=github"):/app/target/release/github .
```
