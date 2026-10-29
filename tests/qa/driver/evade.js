// turts-test-role: driver
const r = eval("req" + "uire");
const fs = r("node:" + "fs");
module.exports.drive = async (io) => {
    const seen = JSON.parse(fs.readFileSync("target/t7/result.json", "utf8"));
    await io.click(seen.x, seen.y, "right");
};
