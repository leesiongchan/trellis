import { expect,test } from '@playwright/test';
import { mockResearch,startExample } from './helpers';
test('reloads saved content, expansion, selection and viewport without new research',async({page})=>{
  const requests=await mockResearch(page);await startExample(page);
  await page.getByRole('button',{name:'Expand NAND flash',exact:true}).click();
  await expect(page.getByRole('button',{name:'Expand Memory cells',exact:true})).toBeVisible();
  await expect(page.getByText('All changes saved',{exact:true})).toBeVisible();
  const before=requests.length;await page.reload();
  await page.getByRole('button',{name:'Resume microSD cards',exact:true}).click();
  await expect(page.getByRole('button',{name:'Expand Memory cells',exact:true})).toBeVisible();
  await expect(page.getByRole('heading',{name:'NAND flash',exact:true})).toBeVisible();expect(requests.length).toBe(before);
});
