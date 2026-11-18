'use strict';
async function clickPointer(page, point, button, pause) {
    await page.mouse.move(...point);
    await pause();
    await page.mouse.down({button});
    await pause();
    await page.mouse.up({button});
    await pause();
}
module.exports={clickPointer};
