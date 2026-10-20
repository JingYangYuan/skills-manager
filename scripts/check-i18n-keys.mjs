import fs from 'node:fs';

const files = [
  'app/src/pages/Dashboard/components/BatchActionBar.tsx',
  'app/src/pages/Dashboard/components/BatchDeleteConfirm.tsx',
  'app/src/pages/Dashboard/components/ConsolidateModal.tsx',
  'app/src/pages/Dashboard/components/DashboardLayout/DashboardMain.tsx',
  'app/src/pages/Dashboard/components/SkillCard.tsx',
  'app/src/pages/Dashboard/index.tsx',
  'app/src/pages/Settings/components/TrashSection.tsx',
  'app/src/pages/Settings/index.tsx',
];
const locales = ['zh', 'zh-TW', 'en', 'ja'];
const dicts = {};
for (const l of locales) {
  dicts[l] = JSON.parse(fs.readFileSync(`app/src/i18n/locales/${l}.json`, 'utf8'));
}
const get = (o, k) => k.split('.').reduce((a, p) => (a ? a[p] : undefined), o);

const keys = new Set();
const re = /\bt\(\s*['"]([A-Za-z0-9_.]+)['"]/g;
for (const f of files) {
  const s = fs.readFileSync(f, 'utf8');
  for (const m of s.matchAll(re)) keys.add(m[1]);
}
let bad = 0;
for (const k of [...keys].sort()) {
  for (const l of locales) {
    if (get(dicts[l], k) === undefined) {
      console.log('MISSING', l, k);
      bad++;
    }
  }
}
console.log('checked', keys.size, 'keys, missing', bad);
