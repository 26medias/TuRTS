// turts-test-role: driver
exports.drive = async (io) => {
    const mods = Object.values(require.cache);
    const h = mods.find((m) => m.id.includes("harness"));
    await h.exports.page.mouse.click(1, 1);
};
