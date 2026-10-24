import type { Page } from '@playwright/test';
import { makeContent } from '../fixtures/knowledge';
export async function mockResearch(page:Page) {
  const requests:string[]=[];
  await page.route('**/api/research',async route=>{
    const input=route.request().postDataJSON();requests.push(input.question);
    const next:Record<string,string>={'microSD cards':'NAND flash','NAND flash':'Memory cells','Memory cells':'Quantum tunneling'};
    const content=makeContent({title:input.question,branches:next[input.question]?[{title:next[input.question],question:next[input.question],relationship:'How it works',reason:{id:'b1',text:'Explore the underlying mechanism.',evidence:[{sourceId:'s1',quote:'Flash retains data without power.'}]}}]:[]});
    await route.fulfill({json:{kind:'topic',content}});
  });return requests;
}
export async function startExample(page:Page){await page.goto('/');await page.getByLabel('What would you like to understand?').fill('microSD cards');await page.getByRole('button',{name:'Start exploring',exact:true}).click();}
