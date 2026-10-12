/**
 * [INPUT]: LobeHub 五项品牌 SVG 与共享双语安装/调用模板。
 * [OUTPUT]: agents 选项和 installationPrompt；指定各工具的真实个人目录与调用入口；通用项让当前工具选择目录。
 * [POS]: 网络页纯数据/文案边界；只描述完整 Skill 的安装与运行，具体收集/验证/部署协议归 Skill。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import generic from '../assets/ai-icons/generic.svg';
import codex from '../assets/ai-icons/codex.svg';
import cursor from '../assets/ai-icons/cursor.svg';
import opencode from '../assets/ai-icons/opencode.svg';
import copilot from '../assets/ai-icons/copilot.svg';
import {translate,type Locale} from './i18n';
export const agents=[
    {id:'generic',name:'General',icon:generic,directory:'',invoke:'ccbaga-network'},
    {id:'codex',name:'Codex',icon:codex,directory:'~/.agents/skills/ccbaga-network',invoke:'$ccbaga-network'},
    {id:'cursor',name:'Cursor',icon:cursor,directory:'~/.cursor/skills/ccbaga-network',invoke:'/ccbaga-network'},
    {id:'opencode',name:'OpenCode',icon:opencode,directory:'~/.config/opencode/skills/ccbaga-network',invoke:'/ccbaga-network'},
    {id:'copilot',name:'Copilot',icon:copilot,directory:'~/.copilot/skills/ccbaga-network',invoke:'/ccbaga-network'},
];
export function installationPrompt(id:string,locale:Locale,localSource?:string|null):string {
    const agent=agents.find(item=>item.id===id)||agents[0];
    if(localSource) return translate('请将本机目录 {v0} 中完整的 Windows 兼容网络 Skill 安装到 {v1}，供 {v2} 使用。安装后运行 {v3}，使用中文。',
        {v0:JSON.stringify(localSource),v1:agent.directory||translate('当前 AI 工具的 Skill 目录',{},locale),v2:agent.id==='generic'?translate('当前 AI 工具',{},locale):agent.name,v3:agent.invoke},locale);
    return translate('请将 {v0} 中完整的 skills/ccbaga-network 安装到 {v1}，供 {v2} 使用。安装后运行 {v3}，使用中文。',
        {v0:'https://github.com/liseami/CCBaga',v1:agent.directory||translate('当前 AI 工具的 Skill 目录',{},locale),v2:agent.id==='generic'?translate('当前 AI 工具',{},locale):agent.name,v3:agent.invoke},locale);
}
