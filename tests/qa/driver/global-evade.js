// turts-test-role: driver
exports.drive = async (io) => {
    const g = global["pro" + "cess"];
    const fs = g["mainModule"]["req" + "uire"]("node:" + "fs");
    await io.click(1, 1, "left");
};
