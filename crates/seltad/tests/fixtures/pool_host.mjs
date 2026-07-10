// An app-provided pool host (docs/05 §websocket): dials in over the Node 22
// global WebSocket — no SDK, no dependencies, just the protocol. Provides a
// deterministic `parity` verifier.
const url = process.argv[2];
if (!url) {
  console.error("usage: node pool_host.mjs ws://host/pools/{pool}/hosts/connect");
  process.exit(2);
}

const ws = new WebSocket(url);
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  const reply = (result) =>
    ws.send(JSON.stringify({ jsonrpc: "2.0", id: message.id, result }));
  if (message.method === "initialize") {
    reply({
      host: { name: "parity-pool-host", version: "0.0.1" },
      extensions: [
        {
          name: "parity",
          determinism: "deterministic",
          config_schema: null,
          needs: [],
          settings_schema: null,
          delta_schema: null,
        },
      ],
    });
  } else if (message.method === "verify") {
    const n = message.params.value;
    if (typeof n === "number" && n % 2 === 0) {
      reply({ verdict: "pass" });
    } else {
      reply({
        verdict: "fail",
        delta: { message: `expected an even number, got ${JSON.stringify(n)}` },
      });
    }
  } else if (message.method === "shutdown") {
    reply({});
    ws.close();
  }
};
ws.onclose = () => process.exit(0);
ws.onerror = () => process.exit(1);
