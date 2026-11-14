# Shared skirmish host

Status: SHARE1–SHARE5 pass. The temporary public test links are closed.

## Player result

One command starts a human match and prints an HTTPS address for a second player.
Each player opens the address in a WebGPU-capable browser.
Keep the local computer game as the default.
Keep the existing local host and join mode.

## Delivery

Use the prepared client, package, and native authoritative server.
Do not rebuild the game when the hosting command starts. Refuse stale prepared inputs.
Use temporary Cloudflare Quick Tunnels for this development build.
Use separate HTTPS and WSS tunnels for the existing separate native ports.
No new proxy or changes to the game wire protocol are necessary.
Set the advertised package origin to the public HTTPS origin before boot.
All browser-facing fetches and the WebSocket use TLS.
Keep native listeners on loopback. Do not open firewall ports or change system services.
Do not read or change the operator's Cloudflare configuration.
The script owns and stops only its own server and tunnel processes.
Print the sharing address only after all services are ready.
Say that the link is temporary and that anyone with the link can join.
Keep server logs and stop the tunnels when the command ends.
Do not expose the local New skirmish restart host.
Use a separate explicit join mode for a fresh browser at an HTTPS address.
Every agent browser uses a fresh profile and `--alsa-output-device=null`.

## Acceptance

| Row | Check |
| --- | --- |
| SHARE1 | A fake tunnel executable proves startup ordering, public package URLs, readiness, failures, and child cleanup. |
| SHARE2 | URL validation accepts HTTPS join links and loopback development links. Reject invalid URLs and credentials. |
| SHARE3 | A real browser loads the public HTTPS address, fetches the hash-verified package, joins through WSS, and finishes an ordinary order. |
| SHARE4 | Two browsers join the shared match as different slots. Both observe an ordinary player order. |
| SHARE5 | On shutdown, both tunnel processes, the native server, and owned test profiles are gone. |

A temporary link does not establish production hosting or hosted service reliability.
Ready lobby, shared rematch, reconnect identity, and a full match played by two people remain separate work.

Quick Tunnels are documented at https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/.
