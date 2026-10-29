// turts-test-role: driver
const png = require("./png-pure.js");
exports.drive = async (io) => { const b = await io.screenshot(); await io.save("a.png", b); png.decode(b); };
