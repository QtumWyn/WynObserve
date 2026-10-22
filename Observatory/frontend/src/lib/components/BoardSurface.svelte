<script lang="ts">
  import type { motherboardLayout } from '../data/machine';
  import type { Motherboard } from '../data/telemetry';
  let { board, identity, paused = false }: { board: ReturnType<typeof motherboardLayout>; identity?: Motherboard | null; paused?: boolean } = $props();
  const uid = $props.id();
  let portable = $derived(board.profile.family === 'portable');
  let cpu = $derived(board.nodes.find(n => n.kind === 'CPU')!);
  let ram = $derived(board.nodes.find(n => n.kind === 'MEMORY')!);
</script>
<svg class="pcb" viewBox={`0 0 ${board.width} ${board.height}`} aria-hidden="true">
  <defs>
    <linearGradient id={`${uid}-board`} x1="0" y1="0" x2="1" y2="1"><stop stop-color={board.profile.matched ? '#303235' : '#253333'}/><stop offset=".5" stop-color={board.profile.matched ? '#1a2024' : '#152126'}/><stop offset="1" stop-color={board.profile.matched ? '#252a2c' : '#10191e'}/></linearGradient>
    <linearGradient id={`${uid}-sink`} x1="0" y1="0" x2="1" y2="1"><stop stop-color="#647078"/><stop offset=".5" stop-color="#2d3741"/><stop offset="1" stop-color="#141c25"/></linearGradient>
    <linearGradient id={`${uid}-silver`} x1="0" y1="0" x2="1" y2="1"><stop stop-color="#c3cece"/><stop offset=".4" stop-color="#707e85"/><stop offset=".65" stop-color="#c4cbcb"/><stop offset="1" stop-color="#5b676e"/></linearGradient>
    <pattern id={`${uid}-vias`} width="24" height="24" patternUnits="userSpaceOnUse"><circle cx="4" cy="4" r="1.2" fill="#83998b" opacity=".13"/></pattern>
  </defs>
  <path d={`M40 14H853L884 45V${board.pcbHeight-40}L860 ${board.pcbHeight-14}H38L14 ${board.pcbHeight-38}V40Z`} fill={`url(#${uid}-board)`} stroke="#65766e" stroke-width="3"/>
  <path d={`M42 23H850L875 48V${board.pcbHeight-44}L854 ${board.pcbHeight-23}H42L23 ${board.pcbHeight-42}V44Z`} fill={`url(#${uid}-vias)`} stroke="#35443e"/>
  {#if board.profile.matched}
    <path d="M90 690L650 335H709L90 737Z M192 803L791 420V473L254 820Z M410 820L795 571V630L503 820Z" fill="#aeb8b9" opacity=".2"/>
  {/if}
  {#each Array.from({length: 12}, (_,i)=>i) as i}
    <path d={`M${cpu.x+i*6} ${cpu.y}V${75-i*3}H${556+i*5}L${588+i*5} ${108-i*3}V${board.pcbHeight-180+i*4}H${690+i*8}V${board.pcbHeight-100+i*3}`} class="copper"/>
    <path d={`M${cpu.x-i*6} ${cpu.y+140}H${230-i*5}V${board.pcbHeight-118+i*4}H${640+i*9}`} class="copper"/>
  {/each}
  {#each [[42,42],[855,42],[42,board.pcbHeight-42],[855,board.pcbHeight-42],[854,board.pcbHeight*.53],[42,board.pcbHeight*.53]] as hole}
    <circle cx={hole[0]} cy={hole[1]} r="11" fill="#9b926b" stroke="#42584b" stroke-width="4"/><circle cx={hole[0]} cy={hole[1]} r="5" fill="#080d10" stroke="#b7b18b"/>
  {/each}
  <text x="300" y="55" class="brand">{(identity?.model ?? 'WYN / OBSERVATORY').slice(0,48)}</text>
  <text x="300" y="74" class="silk">{board.profile.matched ? 'MODEL PROFILE · MICRO-ATX' : `${board.profile.family.toUpperCase()} · ADAPTIVE SCHEMATIC`}</text>
  <!-- Rear I/O and VRM materials remain illustrative; firmware does not describe their outlines. -->
  <rect x="30" y="100" width="100" height={portable ? 180 : 162} rx="5" fill={`url(#${uid}-sink)`} stroke="#7b898a"/>
  {#each [0,1] as i}
    <rect x="36" y={112+i*64} width="86" height="50" rx="2" fill={`url(#${uid}-silver)`} stroke="#b6c3c5"/>
    <rect x="44" y={121+i*64} width="60" height="14" rx="2" fill="#0b1216"/><rect x="44" y={141+i*64} width="60" height="14" rx="2" fill="#0b1216"/>
    <path d={`M50 ${128+i*64}h40 M50 ${148+i*64}h40`} stroke={i === 1 ? '#406aa0' : '#a0a7a2'} stroke-width="4"/>
  {/each}
  <rect x={cpu.x-110} y={cpu.y-12} width="65" height={cpu.height+10} rx="5" fill={`url(#${uid}-sink)`} stroke="#64737d"/>
  {#each Array.from({length:12},(_,i)=>i) as i}<path d={`M${cpu.x-102} ${cpu.y+i*17}h49`} stroke="#667780" stroke-width="3" opacity=".65"/>{/each}
  <rect x={cpu.x-12} y={cpu.y-52} width={cpu.width+20} height="29" rx="3" fill={`url(#${uid}-sink)`} stroke="#6b7880"/>
  {#each [0,1,2,3,4,5,6,7] as i}
    <rect x={cpu.x-6+i*28} y={cpu.y-48} width="6" height="21" fill="#83918f" opacity=".35"/>
    <rect x={cpu.x-35} y={cpu.y+6+i*23} width="23" height="18" rx="3" fill="#64716c" stroke="#98a29b"/>
    <circle cx={cpu.x+cpu.width+20} cy={cpu.y+10+i*25} r="6" fill={`url(#${uid}-silver)`} stroke="#303e43" stroke-width="2"/>
  {/each}
  <text x={cpu.x+10} y={cpu.y+cpu.height+18} class="silk">{board.profile.socket ?? (portable ? 'PROCESSOR PACKAGE' : 'CPU / PACKAGE')}</text>
  <text x={ram.x} y={ram.y-12} class="silk">{board.memorySlots.length ? `${board.memorySlots.length} MEMORY POSITIONS` : 'MEMORY · INVENTORY UNKNOWN'}</text>
  {#if !portable}
    <rect x="818" y="142" width="38" height="186" rx="3" fill="#a6aaa2" stroke="#d1cfb9"/>
    {#each Array.from({length:12},(_,i)=>i) as i}<rect x="824" y={149+i*14} width="10" height="9" fill="#172122"/><rect x="840" y={149+i*14} width="10" height="9" fill="#172122"/>{/each}
    <path d="M856 200h7v66h-7" fill="#9fa391" stroke="#d1cfb9"/><text x="822" y="344" class="silk">ATX</text>
    <rect x="185" y="37" width="88" height="30" rx="3" fill="#1b2429" stroke="#83918c"/>
    {#each [0,1,2,3,4,5,6,7] as i}<rect x={191+i*9} y="44" width="5" height="16" fill="#acaa8c"/>{/each}
    <text x="188" y="81" class="silk">CPU POWER</text>
  {/if}
  <!-- Only a matched profile asserts connector count; unknown boards get no invented slots. -->
  {#each board.profile.pcieLabels as label, i}
    <rect x="178" y={554+i*79} width="448" height="27" rx="3" fill={i === 0 ? '#a6b0af' : '#333e43'} stroke="#88968e"/>
    <path d={`M186 ${566+i*79}h419`} stroke="#0b1718" stroke-width="7"/><rect x="626" y={555+i*79} width="18" height="25" fill="#80938a"/>
    <text x="185" y={547+i*79} class="silk">{label}</text>
  {/each}
  {#if board.profile.matched}
    <text x="250" y="427" class="silk">{board.profile.m2Labels[0]} · STORAGE OVERLAY</text>
    <g transform="translate(630 520) rotate(90)">
      <rect width="290" height="65" rx="4" fill="#142326" stroke="#718f81" stroke-dasharray="3 5"/>
      <rect x="272" y="5" width="22" height="55" fill="#1b1d22" stroke="#6e7b72"/>
      <circle cx="16" cy="32" r="7" fill="#a2ad9b"/><text x="40" y="37" class="silk">{board.profile.m2Labels[1]}</text>
    </g>
    {#each Array.from({length:board.profile.sataPorts},(_,i)=>i) as i}
      <rect x={808+(i%2)*22} y={686+Math.floor(i/2)*43} width="21" height="37" rx="2" fill="#283239" stroke="#89968c"/>
      <path d={`M${814+(i%2)*22} ${693+Math.floor(i/2)*43}v21h10`} stroke="#a59567" stroke-width="3" fill="none"/>
    {/each}
    <text x="807" y="781" class="silk">SATA3</text>
  {/if}
  {#if !portable}
    <rect x="700" y={board.pcbHeight-235} width="100" height="96" rx="5" fill={`url(#${uid}-sink)`} stroke="#61727b"/>
    {#each Array.from({length:7},(_,i)=>i) as i}<path d={`M${708+i*13} ${board.pcbHeight-226}v76`} stroke="#7a8b92" stroke-width="3" opacity=".4"/>{/each}
    <text x="750" y={board.pcbHeight-181} text-anchor="middle" class="silk">{board.profile.matched ? 'B860' : 'CHIPSET'}</text>
    <circle cx="825" cy="400" r="33" fill="#10191d" stroke="#5c6e66" stroke-width="4"/><circle cx="825" cy="400" r="27" fill={`url(#${uid}-silver)`} stroke="#bdc6c4"/>
    <path d="M817 392h16 M825 384v16" stroke="#647578" stroke-width="2"/><text x="825" y="416" text-anchor="middle" class="battery">CR2032</text>
  {/if}
  {#each Array.from({length:18},(_,i)=>i) as i}
    <g transform={`translate(${62+(i%4)*23} ${board.pcbHeight-220+Math.floor(i/4)*25})`}>
      <rect width="15" height="7" fill="#b0a992"/><rect x="3" y="-1" width="9" height="9" fill="#3e3931"/><circle cx="18" cy="12" r="2" fill="#9eb0a4"/>
    </g>
  {/each}
  {#each [0,1,2,3] as port}
    <rect x={292+port*120} y={board.pcbHeight-70} width="82" height="24" rx="2" fill="#121d20" stroke="#6d7e75"/>
    {#each [0,1,2,3,4,5,6,7] as pin}<path d={`M${300+port*120+pin*9} ${board.pcbHeight-65}v14`} stroke="#bdab6e" stroke-width="2"/>{/each}
  {/each}
  <text x="292" y={board.pcbHeight-30} class="silk">BOARD HEADERS · ILLUSTRATIVE</text>
  <g class="power-led" class:held={paused}><circle cx="832" cy="458" r="5"/><text x="814" y="479" class="silk">{paused ? 'HOLD' : 'FEED'}</text></g>
  {#if board.hasDetails}
    <rect x="920" y="80" width="272" height={board.nodes.filter(n => n.detached).length*150+65} rx="12" fill="#1a25304d" stroke="#4c6263" stroke-dasharray="4 8"/>
    <text x="938" y="104" class="silk">ENLARGED HARDWARE DETAILS</text>
    {#each board.nodes.filter(n => n.detached) as node}
      <text x="1056" y={node.y+node.height+22} text-anchor="middle" class="silk">{node.kind === 'NPU' || node.integrated === true ? 'PACKAGE / ACCELERATOR' : 'ADDITIONAL ADAPTER'}</text>
    {/each}
  {/if}
</svg>
<style>
  .pcb { position: absolute; width: 100%; height: 100%; pointer-events: none; }
  .copper { stroke: #719078; stroke-width: 1; opacity: .22; fill: none; }
  .brand { fill: #c2cec8; font: 600 15px 'JetBrains Mono', monospace; letter-spacing: 1.4px; }
  .silk { fill: #93aaa1; font: 8px 'JetBrains Mono', monospace; letter-spacing: .7px; }
  .battery { fill: #43585b; font: 7px 'JetBrains Mono', monospace; }
  .power-led circle { fill: #7ee6aa; filter: drop-shadow(0 0 4px #7ee6aa); }
  .power-led.held circle { fill: #bdad74; filter: none; }
</style>
