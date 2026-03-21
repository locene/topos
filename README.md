# Topos

A convenient [TROW](https://trow.cc) site search engine, powered by full-text search for an accelerated experience.

**Try it now:** https://topos.locene.com

<table>
    <tr>
        <td><img align="center" src="https://locene.com/repos/topos/assets/001.png" /></td>
        <td><img align="center" src="https://locene.com/repos/topos/assets/002.png" /></td>
        <td><img align="center" src="https://locene.com/repos/topos/assets/003.png" /></td>
    </tr>
</table>


## Tech Stack 

| Layer      | Tech                                                                                                                                                                                                   |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Backend    | [TROW API](https://trow.cc/wiki/trow/api/start) · [Meilisearch](https://github.com/meilisearch/meilisearch) · [Axum](https://github.com/tokio-rs/axum) · [Valkey](https://github.com/valkey-io/valkey) |
| Frontend   | [Yew](https://github.com/yewstack/yew)                                                                                                                                                                 |
| CI / CD    | GitHub Actions                                                                                                                                                                                         |
| Deployment | Docker · Debian (trixie-slim) · Nginx (alpine-slim)                                                                                                                                                    |


## To-Do

- [ ] Implement [Change Data Capture](https://en.wikipedia.org/wiki/Change_data_capture) (CDC) mechanism to synchronize updated posts (pending TROW support)


## How to Build

### Prerequisites

| Name                                                      | Version | Description             |
| --------------------------------------------------------- | ------- | ----------------------- |
| Rust                                                      | 1.92.0+ | Programming Language    |
| [Meilisearch](https://github.com/meilisearch/meilisearch) | Latest  | Full-Text Search Engine |
| [Valkey](https://github.com/valkey-io/valkey)             | Latest  | Cache                   |

### Clone the Repository

First, clone the project repository to your local machine:

```Bash
git clone https://github.com/locene/topos.git
cd topos
```

### Configure and Run

#### Backend

```Bash
cd backend
```

Modify the ```.env.example``` file, renaming it to ```.env``` or ```.env.development.local``` based on your needs.

Here's an explanation of several key variables:

| Name               | Description                                                                                                                 |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| TROW_API_URL       | TROW API endpoint. See [documentation](https://trow.cc/wiki/trow/api/start) for the URL, usually ```https://api.trow.cc```. |
| TROW_CLIENT_ID     | The Client ID for your TROW application. Obtain this by applying through TROW.                                              |
| TROW_CLIENT_SECRET | The Client Secret for your TROW application. Obtain this by applying through TROW.                                          |
| ADMIN_TOKEN        | Token required for accessing backend Admin APIs.                                                                            |

After configuring the environment variables, run ```cargo run``` in the ```backend``` root directory.

#### Webapp

```Bash
cd webapp
```

Modify ```src/config/config_dev.rs``` and ```src/config/config_prod.rs``` as needed.

Then, run ```cargo run``` in the ```webapp``` root directory.

Once it's running, simply open your browser and navigate to ```http://127.0.0.1:8080```.


## Contributor Notes

This project utilizes a high-privilege API provided by TROW (with extended rate limits). Consequently, TROW requires contributors to sign a [Contributor License Agreement](https://trow.cc/contributor-license-agreement). All submissions to this repository are subject to the terms of this agreement.


## License

[AGPL-3.0](LICENSE)