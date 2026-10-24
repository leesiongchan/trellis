import { expect,test } from '@playwright/test';
import { mockResearch,startExample } from './helpers';
test('phone switches between reader and map without losing the selected branch',async({page})=>{
 await page.setViewportSize({width:390,height:844});await mockResearch(page);await startExample(page);
 await page.getByRole('button',{name:'Read NAND flash',exact:true}).click();
 await expect(page.getByRole('heading',{name:'NAND flash',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Map',exact:true}).click();await expect(page.getByRole('region',{name:'Topic map',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Expand NAND flash',exact:true}).click();
 await expect(page.getByRole('button',{name:'Read Memory cells',exact:true})).toBeVisible();
 await page.getByRole('button',{name:'Read',exact:true}).click();await expect(page.getByRole('heading',{name:'NAND flash',exact:true})).toBeVisible();
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
});
test('keyboard can select, expand and refocus topics',async({page})=>{
 await mockResearch(page);await startExample(page);const expand=page.getByRole('button',{name:'Expand NAND flash',exact:true});await expand.focus();await page.keyboard.press('Enter');
 await expect(page.getByRole('button',{name:'Read Memory cells',exact:true})).toBeVisible();
 const refocus=page.getByRole('button',{name:'Refocus selected topic'});await refocus.focus();await page.keyboard.press('Enter');await expect(refocus).toBeFocused();
});
