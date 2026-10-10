(() => {
  const ROWS=100, COLS=26;
  let data=[], styles=[], active={r:0,c:0}, selected=new Set(["0,0"]), sheetNames=["Лист1"], currentSheet=0;
  const $=id=>document.getElementById(id);
  const letters=c=>{let s="";for(c++;c;c=Math.floor((c-1)/26))s=String.fromCharCode(65+(c-1)%26)+s;return s};
  const addr=(r,c)=>letters(c)+(r+1);
  function blank(){return Array.from({length:ROWS},()=>Array(COLS).fill(""))}
  function init(){data=[blank()];styles=[Array.from({length:ROWS},()=>Array.from({length:COLS},()=>({})))];render();select(0,0)}
  function cur(){return data[currentSheet]}
  function sty(){return styles[currentSheet]}
  function colNum(s){let n=0;for(const ch of s.toUpperCase())n=n*26+ch.charCodeAt(0)-64;return n-1}
  function getCell(ref){const m=ref.trim().match(/^([A-Z]+)(\d+)$/i);if(!m)return NaN;return numeric(cur()[+m[2]-1]?.[colNum(m[1])]??"")}
  function numeric(v){if(typeof v==="number")return v;if(typeof v!=="string")return NaN;if(v.startsWith("="))return evalFormula(v.slice(1));const n=Number(v.replace(",","."));return v.trim()!==""&&Number.isFinite(n)?n:NaN}
  function rangeVals(ref){const p=ref.split(":");if(p.length===1)return [getCell(p[0])];const a=p[0].match(/^([A-Z]+)(\d+)$/i),b=p[1].match(/^([A-Z]+)(\d+)$/i);if(!a||!b)return [];let vals=[];for(let r=Math.min(+a[2],+b[2])-1;r<=Math.max(+a[2],+b[2])-1;r++)for(let c=Math.min(colNum(a[1]),colNum(b[1]));c<=Math.max(colNum(a[1]),colNum(b[1]));c++)vals.push(numeric(cur()[r]?.[c]??""));return vals.filter(Number.isFinite)}
  function evalFormula(f){
    try{
      let m=f.trim().match(/^(SUM|СУММ|AVERAGE|СРЗНАЧ|МИН|MIN|МАКС|MAX|COUNT|СЧЁТ)\(([^()]*)\)$/i);
      if(m){let v=rangeVals(m[2].replace(/\$/g,""));switch(m[1].toUpperCase()){case"SUM":case"СУММ":return v.reduce((a,b)=>a+b,0);case"AVERAGE":case"СРЗНАЧ":return v.length?v.reduce((a,b)=>a+b,0)/v.length:0;case"MIN":case"МИН":return v.length?Math.min(...v):0;case"MAX":case"МАКС":return v.length?Math.max(...v):0;default:return v.length}}
      }
      // Replace cell references before arithmetic evaluation; no arbitrary JS is evaluated.
      let expr=f.replace(/\$?[A-Z]{1,3}\$?\d+/gi,ref=>{const n=getCell(ref.replace(/\$/g,""));return Number.isFinite(n)?String(n):"0"});
      if(!/^[\d\s.+\-*/()%]+$/.test(expr))return "#ОШИБКА";
      // Arithmetic-only expression.
      return Function('"use strict";return ('+expr+')')();
    }catch(_){return "#ОШИБКА"}
  }
  function display(r,c){const v=cur()[r]?.[c]??"";if(typeof v==="string"&&v.startsWith("=")){const x=evalFormula(v.slice(1));return Number.isFinite(x)?String(Math.round(x*1e10)/1e10):String(x)}return String(v)}
  function render(){const t=$("grid");t.innerHTML="";let tr=document.createElement("tr"),corner=document.createElement("th");corner.className="rowhead";corner.textContent="№";tr.append(corner);for(let c=0;c<COLS;c++){let th=document.createElement("th");th.textContent=letters(c);tr.append(th)}t.append(tr);
    for(let r=0;r<ROWS;r++){let row=document.createElement("tr"),rh=document.createElement("td");rh.className="rownum";rh.textContent=r+1;row.append(rh);
      for(let c=0;c<COLS;c++){let td=document.createElement("td");td.className="cell";td.contentEditable="true";td.dataset.r=r;td.dataset.c=c;td.textContent=display(r,c);let s=sty()[r][c];if(s.bold)td.classList.add("bold");if(s.bg)td.style.background=s.bg;td.addEventListener("focus",()=>select(r,c));td.addEventListener("input",()=>{cur()[r][c]=td.innerText; $("formulaInput").value=cur()[r][c]});td.addEventListener("keydown",e=>{if(e.key==="Enter"){e.preventDefault();cur()[r][c]=td.innerText;select(Math.min(ROWS-1,r+1),c);focusCell()}else if(e.key==="Tab"){e.preventDefault();cur()[r][c]=td.innerText;select(r,Math.min(COLS-1,c+1));focusCell()}});td.addEventListener("blur",()=>{cur()[r][c]=td.innerText;td.textContent=display(r,c)});row.append(td)}t.append(row)}
    renderTabs()
  }
  function select(r,c){active={r,c};$("cellName").textContent=addr(r,c);$("formulaInput").value=cur()[r][c]??"";document.querySelectorAll("td.cell.selected").forEach(x=>x.classList.remove("selected"));const el=document.querySelector(`td.cell[data-r="${r}"][data-c="${c}"]`);if(el)el.classList.add("selected")}
  function focusCell(){const e=document.querySelector(`td.cell[data-r="${active.r}"][data-c="${active.c}"]`);if(e)e.focus()}
  function apply(){cur()[active.r][active.c]=$("formulaInput").value;render();select(active.r,active.c);focusCell();$("status").textContent="Изменение применено"}
  function renderTabs(){const box=$("sheetTabs");box.querySelectorAll("button.sheet").forEach(x=>x.remove());sheetNames.forEach((n,i)=>{let b=document.createElement("button");b.className="sheet"+(i===currentSheet?" active":"");b.textContent=n;b.onclick=()=>{currentSheet=i;render();select(0,0)};box.insertBefore(b,$("addSheet"))})}
  $("applyBtn").onclick=apply;$("formulaInput").addEventListener("keydown",e=>{if(e.key==="Enter")apply()});
  $("newBtn").onclick=()=>{if(confirm("Создать новую книгу? Несохранённые данные будут потеряны.")){init();sheetNames=["Лист1"];currentSheet=0;render()}};
  $("openBtn").onclick=()=>$("fileInput").click();
  $("fileInput").onchange=async e=>{const f=e.target.files[0];if(!f)return;try{if(!window.XLSX)throw Error("Не удалось загрузить библиотеку Excel. Проверьте интернет.");const ab=await f.arrayBuffer();let wb;if(f.name.toLowerCase().endsWith(".csv")){wb=XLSX.read(new TextDecoder().decode(ab),{type:"string"});}else wb=XLSX.read(ab,{type:"array",cellFormula:true});data=[];styles=[];sheetNames=wb.SheetNames.slice();for(const name of sheetNames){const ws=wb.Sheets[name],a=XLSX.utils.sheet_to_json(ws,{header:1,raw:false,defval:""});let d=blank(),s=Array.from({length:ROWS},()=>Array.from({length:COLS},()=>({})));a.slice(0,ROWS).forEach((row,r)=>row.slice(0,COLS).forEach((v,c)=>{const cell=ws[XLSX.utils.encode_cell({r,c})];d[r][c]=cell?.f?"="+cell.f:v}));data.push(d);styles.push(s)}currentSheet=0;render();select(0,0);$("status").textContent="Открыт файл: "+f.name}catch(err){alert("Ошибка открытия: "+err.message)}e.target.value=""};
  $("saveBtn").onclick=()=>{if(!window.XLSX){alert("Библиотека Excel недоступна. Проверьте интернет.");return}const wb=XLSX.utils.book_new();data.forEach((d,i)=>{const aoa=d.map((row,r)=>row.map((v,c)=>typeof v==="string"&&v.startsWith("=")?{f:v.slice(1)}:v));const ws=XLSX.utils.aoa_to_sheet(aoa);XLSX.utils.book_append_sheet(wb,ws,(sheetNames[i]||"Лист"+(i+1)).slice(0,31))});XLSX.writeFile(wb,"RusTab.xlsx");$("status").textContent="Книга подготовлена к скачиванию"};
  $("csvBtn").onclick=()=>{const csv=cur().map(row=>row.map(v=>'"'+String(v).replace(/"/g,'""')+'"').join(";")).join("\r\n");const a=document.createElement("a");a.href=URL.createObjectURL(new Blob(["\ufeff"+csv],{type:"text/csv;charset=utf-8"}));a.download=(sheetNames[currentSheet]||"Лист")+".csv";a.click();URL.revokeObjectURL(a.href)};
  $("boldBtn").onclick=()=>{const s=sty()[active.r][active.c];s.bold=!s.bold;render();select(active.r,active.c)};
  $("fillBtn").onclick=()=>{sty()[active.r][active.c].bg=$("colorPick").value;render();select(active.r,active.c)};
  $("addSheet").onclick=()=>{let name=prompt("Название листа","Лист"+(data.length+1));if(!name)return;sheetNames.push(name.slice(0,31));data.push(blank());styles.push(Array.from({length:ROWS},()=>Array.from({length:COLS},()=>({}))));currentSheet=data.length-1;render();select(0,0)};
  function insertFunc(fn){cur()[active.r][active.c]="="+fn+"("+addr(active.r,active.c)+":"+addr(active.r,active.c)+")";render();select(active.r,active.c);$("formulaInput").focus()}
  $("sumBtn").onclick=()=>insertFunc("СУММ");$("avgBtn").onclick=()=>insertFunc("СРЗНАЧ");$("minBtn").onclick=()=>insertFunc("МИН");$("maxBtn").onclick=()=>insertFunc("МАКС");
  init();
})();
