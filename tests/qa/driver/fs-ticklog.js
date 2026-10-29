// turts-test-role: driver
const fs = require("node:fs");
exports.drive = async (io) => {
    const last = fs.readFileSync("target/t7/server-tick.log", "utf8").trim().split("\n").pop();
    const [, , , x, y] = last.split(",");
    await io.click(Number(x), Number(y), "right");
};
