await page.mouse.move(400,300);
await page.mouse.down();
await page.mouse.move(600,450);
await page.mouse.up();
await page.screenshot({path:"a.png"});
