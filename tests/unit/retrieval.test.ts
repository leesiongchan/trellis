import { expect,it } from 'vitest';
import { parseSearchResults } from '../../src/lib/server/research/tavily';
it('drops unsafe URLs, missing raw content and malformed results; caps evidence size',()=>{
  const result=parseSearchResults({results:[
    {title:'Good',url:'https://example.org',raw_content:'x'.repeat(15000)},
    {title:'Unsafe',url:'javascript:alert(1)',raw_content:'text'},
    {title:'Snippet only',url:'https://example.net',content:'not full evidence'},
    {title:123,url:'https://example.com',raw_content:'bad'}
  ]});
  expect(result).toHaveLength(1);expect(result[0].text).toHaveLength(12000);expect(result[0].id).toBeTruthy();
});
