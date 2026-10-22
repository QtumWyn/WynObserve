import { test } from 'node:test';
import assert from 'node:assert/strict';
import { placePopover } from '../src/lib/data/popover.ts';

test('component popup prefers a nearby free side', () => {
  const bounds={left:0,top:0,right:1000,bottom:800}, size={width:200,height:160};
  assert.deepEqual(placePopover({left:300,top:200,right:400,bottom:300},size,bounds),{left:412,top:170});
  assert.deepEqual(placePopover({left:900,top:200,right:960,bottom:300},size,bounds),{left:688,top:170});
});
test('popup uses below/above near the top/bottom edge when side placement cannot fit', () => {
  const bounds={left:0,top:0,right:1000,bottom:700}, size={width:280,height:220};
  assert.deepEqual(placePopover({left:450,top:10,right:550,bottom:70},size,bounds),{left:360,top:82});
  assert.deepEqual(placePopover({left:450,top:620,right:550,bottom:680},size,bounds),{left:360,top:388});
});
test('popup remains inside a narrow, offset scrolling panel even for partially visible hardware', () => {
  const bounds={left:500,top:72,right:740,bottom:620}, size={width:224,height:300};
  for (const x of [320,480,520,700]) for (const y of [0,100,500,600]) {
    const p=placePopover({left:x,top:y,right:x+160,bottom:y+100},size,bounds);
    assert.equal(p.left,508);
    assert.ok(p.top>=80 && p.top+size.height<=612);
  }
});
test('placement stays bounded across viewport widths, heights and component corners', () => {
  for (const width of [220,390,800,1600]) for (const height of [180,480,1080]) {
    const bounds={left:110,top:72,right:110+width,bottom:72+height};
    const size={width:Math.min(320,width-16),height:Math.min(330,height-16)};
    for (const x of [bounds.left,bounds.right-100]) for (const y of [bounds.top,bounds.bottom-100]) {
      const p=placePopover({left:x,top:y,right:x+100,bottom:y+100},size,bounds);
      assert.ok(p.left>=bounds.left+8 && p.left+size.width<=bounds.right-8);
      assert.ok(p.top>=bounds.top+8 && p.top+size.height<=bounds.bottom-8);
    }
  }
});
