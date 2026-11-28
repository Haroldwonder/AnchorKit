describe("ui package dual ESM/CJS builds", () => {
  it("exposes a consumable CJS build", () => {
    const cjs = require("../../dist/cjs/index.js");
    expect(cjs).toBeDefined();
    expect(typeof cjs).toBe("object");
  });

  it("exposes a consumable ESM build", async () => {
    const esm = await import("../../dist/esm/index.js");
    expect(esm).toBeDefined();
  });
});
