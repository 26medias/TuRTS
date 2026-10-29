// turts-test-role: pure
const zlib = require("node:zlib");
exports.decode = (b) => zlib.inflateSync(b);
