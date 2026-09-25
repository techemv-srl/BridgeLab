import { writeFileSync } from 'node:fs';
import { newSession, ready, sleep } from './lib.mjs';
const d = await newSession();
try { await ready(d); await sleep(2500); writeFileSync('/tmp/claude-0/-home-user-HL7-editor/9ea8d0e7-c124-5677-9811-5f8fd24a4901/scratchpad/main.png', await d.takeScreenshot(), 'base64'); } finally { await d.quit().catch(()=>{}); }
