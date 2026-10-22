<script lang="ts">
  import { intensity } from '../data/machine';
  import type { MachineNode } from '../data/machine';
  import type { BoardMemorySlot } from '../data/motherboard';
  import type { MemoryDevice } from '../data/telemetry';
  let { node, paused = false, slots = [], memoryDevices = [], compactGpu = false }: { node: MachineNode; paused?: boolean; slots?: BoardMemorySlot[]; memoryDevices?: MemoryDevice[]; compactGpu?: boolean } = $props();
  let chips = $derived(memoryDevices.length > 0 && memoryDevices.every(d => [5,10,11,16].includes(d.formFactor ?? 0)));
  let sodimm = $derived(memoryDevices.some(d => d.formFactor === 13) && !memoryDevices.some(d => d.formFactor === 9));
  const uid = $props.id();
  let type = $derived(node.kind === 'GPU' && node.integrated !== true && !compactGpu ? 'card' : node.kind);
  let view = $derived(type === 'CPU' ? '0 0 238 226' : type === 'MEMORY' ? '0 0 166 320' : type === 'card' ? '0 0 440 162' : type === 'NVMe' ? '0 0 300 74' : type === 'NETWORK' ? '0 0 138 166' : '0 0 92 92');
</script>

<svg viewBox={view} preserveAspectRatio={type === 'MEMORY' ? 'none' : 'xMidYMid meet'} aria-hidden="true" class="hardware-art" class:paused>
  <defs>
    <linearGradient id={`${uid}-metal`} x1="0" y1="0" x2="1" y2="1"><stop stop-color="#d2d8d8"/><stop offset=".35" stop-color="#858f95"/><stop offset=".53" stop-color="#b5bec1"/><stop offset="1" stop-color="#555e67"/></linearGradient>
    <linearGradient id={`${uid}-dark`} x1="0" y1="0" x2="0" y2="1"><stop stop-color="#39434c"/><stop offset=".5" stop-color="#1d242c"/><stop offset="1" stop-color="#0e141a"/></linearGradient>
    <linearGradient id={`${uid}-silicon`}><stop stop-color="#172e3c"/><stop offset=".5" stop-color="#547184"/><stop offset="1" stop-color="#233344"/></linearGradient>
    <pattern id={`${uid}-brush`} width="3" height="3" patternUnits="userSpaceOnUse"><path d="M0 1H3" stroke="#fff" opacity=".08" stroke-width=".5"/></pattern>
  </defs>
  {#if type === 'CPU'}
    <rect x="6" y="6" width="220" height="214" rx="9" fill="#070d11" stroke="#536069" stroke-width="2"/>
    <rect x="17" y="16" width="197" height="192" rx="3" fill="#244737" stroke="#618272"/>
    {#each Array.from({length: 16}, (_, i) => i) as i}
      <path d={`M${30+i*11} 20v8 M${30+i*11} 195v8 M21 ${33+i*10}h7 M201 ${33+i*10}h7`} stroke="#b8a064" stroke-width="3"/>
    {/each}
    <path d="M33 34H195V188H33Z M48 49V173H180V49Z" fill={`url(#${uid}-metal)`} fill-rule="evenodd" stroke="#a7b2b5" stroke-width="2"/>
    <path d="M63 49H165V58H180V164H165V173H63V164H48V58H63Z" fill={`url(#${uid}-metal)`} stroke="#d8dfe0"/>
    <path d="M63 49H165V58H180V164H165V173H63V164H48V58H63Z" fill={`url(#${uid}-brush)`}/>
    <text x="114" y="105" class="cpu-print" text-anchor="middle">PROCESSOR</text>
    <text x="114" y="125" class="cpu-print cpu-mark" text-anchor="middle">CPU</text>
    <path d="M74 145H154 M74 149H129 M74 153H143" stroke="#3e494f" opacity=".55"/>
    <path d="M220 37V177Q220 194 232 198V213" stroke="#121a20" stroke-width="7" fill="none"/>
    <path d="M220 37V177Q220 194 232 198V213" stroke="#b7c1c4" stroke-width="4" fill="none"/>
    {#each [[28,28],[202,28],[28,196],[202,196]] as screw}
      <circle cx={screw[0]} cy={screw[1]} r="5" fill="#8d999e"/><path d={`M${screw[0]-3} ${screw[1]}h6`} stroke="#263138"/>
    {/each}
    <path d="M57 58h9v9Z" fill="#e1c36e"/>
  {:else if type === 'MEMORY'}
    {#if !slots.length}
      <rect x="8" y="8" width="150" height="304" rx="6" fill="#17272b" stroke="#81958d" stroke-dasharray="5 7"/>
      <rect x="42" y="106" width="82" height="82" rx="3" fill={`url(#${uid}-dark)`} stroke="#6b8078"/>
      <text x="83" y="151" text-anchor="middle" class="silk">RAM</text>
      <text x="83" y="223" text-anchor="middle" class="silk">INVENTORY</text>
      <text x="83" y="236" text-anchor="middle" class="silk">UNKNOWN</text>
    {:else if chips}
      {#each slots as slot, i}
        <g transform={`translate(${12+(i%2)*76} ${12+Math.floor(i/2)*300/Math.ceil(slots.length/2)})`}>
          <rect width="64" height={Math.min(60, 280/Math.ceil(slots.length/2))} rx="3" fill={`url(#${uid}-dark)`} stroke="#81958d" stroke-dasharray={slot.state === 'unknown' ? '3 4' : undefined}/>
          <text x="32" y="25" text-anchor="middle" class="silk">{slot.state === 'unknown' ? '?' : 'RAM'}</text>
        </g>
      {/each}
    {:else if sodimm}
      {#each slots as slot, i}
        <g transform={`translate(5 ${i*320/slots.length+4}) scale(1 ${Math.min(1, 310/slots.length/74)})`}>
          <rect x="0" y="3" width="156" height="64" rx="3" fill="#10191d" stroke="#8a9d91" stroke-dasharray={slot.state === 'unknown' ? '4 5' : undefined}/>
          {#if slot.state === 'installed'}
            <rect x="7" y="9" width="142" height="47" rx="2" fill="#2c533d" stroke="#738b68"/>
            {#each [0,1,2,3] as chip}<rect x={17+chip*31} y="17" width="24" height="30" fill={`url(#${uid}-dark)`}/>{/each}
            <path d="M11 59h133" stroke="#bfa967" stroke-width="4"/>
          {:else}<text x="78" y="39" class="silk" text-anchor="middle">{slot.state === 'empty' ? 'EMPTY' : '?'}</text>{/if}
          <text x="78" y="76" text-anchor="middle" class="silk">{slot.label.slice(0,20)}</text>
        </g>
      {/each}
    {:else}
      {#each slots as slot, i}
        {@const columns = Math.min(slots.length, 8)}
        {@const rows = Math.ceil(slots.length/8)}
        <g transform={`translate(${(i%columns)*166/columns} ${Math.floor(i/columns)*320/rows}) scale(${166/columns/41} ${1/rows})`}>
          <rect x="2" y="4" width="37" height="300" rx="4" fill="#0a1115" stroke="#81958d" stroke-dasharray={slot.state === 'unknown' ? '5 7' : undefined}/>
          {#if slot.state === 'installed'}
            <rect x="9" y="17" width="23" height="274" rx="2" fill="#224b37" stroke="#608066"/>
            <path d="M10 20v266 M31 20v266" stroke="#b29c58" stroke-width="2"/>
            {#each [0,1,2,3,4,5] as chip}
              <rect x="12" y={29+chip*41} width="17" height="32" rx="2" fill={`url(#${uid}-dark)`} stroke="#45514d" stroke-width=".6"/>
              <path d={`M13 ${36+chip*41}h10 M13 ${40+chip*41}h7`} stroke="#798980" stroke-width=".6" opacity=".55"/>
            {/each}
          {:else if slot.state === 'empty'}<path d="M20 25v260" stroke="#5b5d4c" stroke-width="3"/>
          {:else}<text x="20" y="150" text-anchor="middle" fill="#91a59a" font-size="16">?</text>{/if}
          <path d="M4 7H37V20H28V14H13V20H4Z M4 288H13V294H28V288H37V301H4Z" fill="#a7ada9" stroke="#d1d5cc"/>
          <text x="20" y="319" text-anchor="middle" class="silk">{slot.label.replace(/DDR[0-9]_|DIMM_/g, '').slice(-6)}</text>
        </g>
      {/each}
    {/if}
  {:else if type === 'card'}
    <rect x="18" y="8" width="412" height="142" rx="6" fill="#223e33" stroke="#627b65"/>
    <path d="M40 150H193V158H40Z M209 150H380V158H209Z" fill="#c6ad63"/>
    {#each Array.from({length: 44}, (_, i) => i) as pin}<path d={`M${44+pin*7.5} 150v8`} stroke="#5c502b" stroke-width="1.3"/>{/each}
    <path d="M7 3H20V158H7 M7 26H2V138H7" fill="#828e96" stroke="#bcc5c8" stroke-width="2"/>
    <rect x="23" y="13" width="399" height="130" rx="7" fill={`url(#${uid}-dark)`} stroke="#63717b" stroke-width="2"/>
    {#each [0,1,2,3,4,5,6,7] as fin}<path d={`M35 ${22+fin*16}H410`} stroke="#73818d" opacity=".25"/>{/each}
    <rect x="23" y="13" width="399" height="130" rx="7" fill={`url(#${uid}-brush)`}/>
    <path d="M24 35L65 14H380L421 35 M24 122L60 142H385L421 122" fill="none" stroke="#a1acb5" stroke-width="2" opacity=".45"/>
    {#each [91,223,355] as cx}
      <circle {cx} cy="78" r="55" fill="#090f15" stroke="#475460" stroke-width="3"/>
      <circle {cx} cy="78" r="49" fill="#141c24" stroke="#27333d"/>
      {#each [0,1,2,3,4,5,6,7,8] as blade}
        <path d={`M${cx+9} 74Q${cx+20} 35 ${cx+42} 61Q${cx+30} 59 ${cx+17} 86Z`} transform={`rotate(${blade*40} ${cx} 78)`} fill="#36434e" stroke="#67747e" stroke-width=".6"/>
      {/each}
      <circle {cx} cy="78" r="16" fill={`url(#${uid}-dark)`} stroke="#83909a"/>
      <circle {cx} cy="78" r="8" fill="none" stroke="#738290" opacity=".7"/>
    {/each}
    <path d="M186 12H266" stroke="var(--part-accent)" stroke-width="3"/>
    <rect x="380" y="1" width="32" height="12" rx="2" fill="#0d1418" stroke="#4b5960"/>
    <text x="36" y="136" class="silk">GRAPHICS</text>
  {:else if type === 'NVMe'}
    <path d="M10 8H278L288 17V57L278 66H10V45H4V30H10Z" fill="#214a38" stroke="#78917a" stroke-width="1.5"/>
    <path d="M11 13v16 M11 45v16" stroke="#cfb36c" stroke-width="12"/>
    {#each [0,1,2,3,4,5] as contact}<path d={`M5 ${14+contact*9}H17`} stroke="#6c592f"/>{/each}
    {#each [34,98,162] as x}
      <rect {x} y="17" width="48" height="40" rx="2" fill={`url(#${uid}-dark)`} stroke="#6b7c70"/>
      <path d={`M${x+7} 28h25 M${x+7} 32h16 M${x+7} 36h21`} stroke="#8a9a8f" stroke-width=".7" opacity=".6"/>
    {/each}
    <rect x="225" y="20" width="26" height="30" fill="#b7beb0"/>
    <path d="M231 25v19 M235 25v19 M240 25v19 M244 25v19" stroke="#33473a" stroke-width="2"/>
    <circle cx="278" cy="37" r="9" fill="#a6b0ac" stroke="#d2d7cf"/><path d="M273 37h10 M278 32v10" stroke="#303b3e" stroke-width="2"/>
    <circle cx="264" cy="17" r="2.5" class="activity-led" class:lit={intensity(node.activity) > 0}/>
    <text x="228" y="61" class="silk">M.2</text>
  {:else if type === 'NETWORK'}
    <rect x="12" y="4" width="106" height="76" rx="3" fill={`url(#${uid}-metal)`} stroke="#bac4c7" stroke-width="2"/>
    <rect x="12" y="4" width="106" height="76" rx="3" fill={`url(#${uid}-brush)`}/>
    <path d="M26 17H105V59H82V68H49V59H26Z" fill="#080e12" stroke="#444f54" stroke-width="2"/>
    {#each [0,1,2,3,4,5,6,7] as pin}<path d={`M${37+pin*8} 20v18`} stroke="#c1a460" stroke-width="3"/>{/each}
    <rect x="18" y="63" width="10" height="6" class="link-led" class:lit={intensity(node.activity) > 0}/><rect x="102" y="63" width="10" height="6" class="activity-led" class:lit={intensity(node.activity) > 0}/>
    {#each Array.from({length: 10}, (_,i)=>i) as pin}<path d={`M${36+pin*6} 102v-7 M${36+pin*6} 151v7 M32 ${105+pin*5}h-7 M99 ${105+pin*5}h7`} stroke="#8d9994" stroke-width="2.5"/>{/each}
    <rect x="32" y="101" width="67" height="51" rx="2" fill={`url(#${uid}-dark)`} stroke="#586b63"/>
    <circle cx="39" cy="108" r="2" fill="#98a7a0"/><text x="66" y="132" text-anchor="middle" class="silk">LAN</text>
    <path d="M46 81V94 M58 81V94 M72 81V94 M86 81V94" stroke="#638976"/>
  {:else}
    <rect x="7" y="7" width="78" height="78" rx="4" fill="#294c3b" stroke="#728b73"/>
    {#each Array.from({length: 9}, (_,i)=>i) as pin}<path d={`M${18+pin*7} 8v-5 M${18+pin*7} 84v5 M8 ${18+pin*7}h-5 M84 ${18+pin*7}h5`} stroke="#c4b376" stroke-width="3"/>{/each}
    <rect x="17" y="17" width="58" height="58" rx="2" fill={`url(#${uid}-dark)`} stroke="#63706a"/>
    <rect x="27" y="27" width="38" height="28" fill={`url(#${uid}-silicon)`} stroke="#8babb3"/>
    <path d="M31 30v20 M37 30v20 M43 30v20 M49 30v20 M55 30v20 M61 30v20 M30 38H62 M30 44H62" stroke="#97b8c5" opacity=".22" stroke-width=".7"/>
    <circle cx="23" cy="23" r="2" fill="#b9c4b0"/>
    <text x="46" y="68" class="silk" text-anchor="middle">{type === 'NPU' ? 'NPU' : node.integrated === true ? 'iGPU' : 'GPU'}</text>
  {/if}
</svg>
<style>
  .link-led, .activity-led { fill: #3b453c; }
  .link-led.lit { fill: #88dca6; filter: drop-shadow(0 0 2px #88dca6); }
  .activity-led.lit { fill: #d4bb74; animation: activity-led var(--pulse-duration,2s) ease-in-out infinite alternate; }
  .paused .activity-led { animation-play-state: paused; }
  @keyframes activity-led { from { opacity: .25; } to { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .activity-led.lit { animation: none; } }
  .hardware-art { width: 100%; height: 100%; display: block; overflow: visible; }
  .silk { fill: #b8c7c0; font: 7px 'JetBrains Mono', monospace; letter-spacing: 1px; }
  .cpu-print { fill: #35444d; font: 7px 'JetBrains Mono', monospace; letter-spacing: 1.8px; }
  .cpu-mark { font-size: 15px; font-weight: 700; letter-spacing: 3px; }
</style>
