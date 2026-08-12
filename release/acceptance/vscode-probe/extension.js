const vscode = require("vscode");

const RESULT_URI = vscode.Uri.file(
  "/home/agentmage-test/acceptance/provider-result.json",
);
const EXPECTED = Object.freeze({
  id: "secure-local-read",
  vendor: "agentmage",
  family: "agentmage-secure-read",
  version: "0.0.0-phase9",
  tokenCount: 4,
});

async function activate() {
  let result;
  try {
    const model = await waitForModel();
    const tokenCount = await model.countTokens("acceptance-probe");
    const observed = {
      id: model.id,
      vendor: model.vendor,
      family: model.family,
      version: model.version,
      tokenCount,
    };
    if (JSON.stringify(observed) !== JSON.stringify(EXPECTED)) {
      throw new Error("probe.contract_mismatch");
    }
    result = { schemaVersion: 1, status: "pass", observed };
  } catch (error) {
    result = {
      schemaVersion: 1,
      status: "fail",
      code: error instanceof Error ? error.message : "probe.unknown_failure",
    };
  }
  await vscode.workspace.fs.writeFile(
    RESULT_URI,
    Buffer.from(`${JSON.stringify(result)}\n`, "utf8"),
  );
}

async function waitForModel() {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    const models = await vscode.lm.selectChatModels({
      vendor: EXPECTED.vendor,
      family: EXPECTED.family,
      silent: true,
    });
    if (models.length === 1 && models[0] !== undefined) {
      return models[0];
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("probe.model_unavailable");
}

module.exports = { activate };
