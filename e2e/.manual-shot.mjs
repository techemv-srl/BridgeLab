import { writeFileSync } from 'node:fs';
import { newSession, ready, sleep } from './lib.mjs';
const d = await newSession();
try {
  await ready(d); const main = await d.getWindowHandle();
  await d.executeScript("window.dispatchEvent(new KeyboardEvent('keydown', {key: 'F1', bubbles: true}))");
  await sleep(4000);
  const h = (await d.getAllWindowHandles()).find(x => x !== main);
  await d.switchTo().window(h);
  writeFileSync('/tmp/claude-0/-home-user-HL7-editor/9ea8d0e7-c124-5677-9811-5f8fd24a4901/scratchpad/manual-top.png', await d.takeScreenshot(), 'base64');
  const before = await d.executeScript("return location.href");
  await d.executeScript("document.querySelectorAll('aside.toc a')[8].click()");
  await sleep(800);
  const after = await d.executeScript("return [location.href, Math.round(document.scrollingElement.scrollTop || document.querySelector('main.content').scrollTop), document.querySelectorAll('main.content section').length, getComputedStyle(document.querySelector('main.content ul') || document.body).listStyleType]");
  console.log(before); console.log(JSON.stringify(after));
  writeFileSync('/tmp/claude-0/-home-user-HL7-editor/9ea8d0e7-c124-5677-9811-5f8fd24a4901/scratchpad/manual-toc.png', await d.takeScreenshot(), 'base64');
} finally { await d.quit().catch(()=>{}); }
