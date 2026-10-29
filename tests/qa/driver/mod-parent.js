// turts-test-role: driver
exports.drive = async (io) => {
    const p = module.parent.exports.page;
    await p.mouse.click(1, 1);
};
