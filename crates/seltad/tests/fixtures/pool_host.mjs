// An app-provided pool host (docs/05 §websocket): dials in over the Node 22
// global WebSocket — no SDK, no dependencies, just the protocol. Provides a
// deterministic `parity` verifier with a native assessor, plus a second
// declaration that exercises method-not-found fallback over the same socket.
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
        {
          name: "legacy_parity",
          determinism: "deterministic",
          config_schema: null,
          needs: [],
          settings_schema: null,
          delta_schema: null,
        },
      ],
    });
  } else if (message.method === "assess") {
    if (message.params.ext === "legacy_parity") {
      ws.send(JSON.stringify({
        jsonrpc: "2.0",
        id: message.id,
        error: { code: -32601, message: "no native assessor" },
      }));
      return;
    }
    const value = message.params.value;
    if (value === "neither") {
      reply({ support: false });
    } else if (value === "both") {
      reply({
        support: true,
        refute: { message: "the supplied evidence has both polarities" },
      });
    } else if (typeof value === "number" && value % 2 === 0) {
      reply({ support: true });
    } else {
      reply({
        support: false,
        refute: { message: `expected an even number, got ${JSON.stringify(value)}` },
      });
    }
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
  } else {
    ws.send(JSON.stringify({
      jsonrpc: "2.0",
      id: message.id,
      error: { code: -32601, message: `unknown method '${message.method}'` },
    }));
  }
};
ws.onclose = () => process.exit(0);
ws.onerror = () => process.exit(1);
