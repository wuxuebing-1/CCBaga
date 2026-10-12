/**
 * [INPUT]: 固定资源入口、五项 LobeHub 图标与按需服务规格、Cromma Tabs/Textarea、精简双语提示词与原生剪贴板。
 * [OUTPUT]: Laper 满宽细线三步入口与斜纹分隔、五项 AI 图标分段与问号规格说明、点阵标题和双语提示词一键复制。
 * [POS]: 网络资源导航；应用不收集密钥或执行远端部署，Skill 在用户选定的 AI 工具内运行。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {Button,Tabs,Textarea} from 'cromma-ui';
import {Server as ServerIcon,HomeDefault as HomeIcon} from 'cromma-ui/pika';
import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {useI18n} from './i18n';
import {AppDialog} from './dialog';
import {agents,installationPrompt} from './network-skill';
export function VpsPanel() {
    const {t,locale}=useI18n();
    const [selected,setSelected]=useState('generic'), [copied,setCopied]=useState(''), [error,setError]=useState('');
    const [guide,setGuide]=useState<'dmit'|'webshare'|null>(null);
    const [source,setSource]=useState<string|null>();
    useEffect(()=>{let live=true;void invoke<string|null>('network_skill_source').then(value=>{if(live)setSource(value);}).catch(e=>{if(live)setError(String(e));});return()=>{live=false;};},[]);
    const prompt=source===undefined?'':installationPrompt(selected,locale,source);
    async function open(provider:'dmit'|'webshare'|'skill') {
        if(provider==='skill'&&source){await copy();return;}
        if(provider==='skill'&&source===undefined)return;
        setError('');
        try { await invoke('network_open_resource',{provider}); }
        catch (e) { setError(String(e)); }
    }
    async function copy() {
        if(source===undefined)return;
        setError('');
        try { await invoke('copy_result',{text:prompt});setCopied(prompt); }
        catch(e) { setError(String(e)); }
    }
    return <section className="system-page network-page"><div className="cleanup-title"><h1>{t('网络环境')}</h1></div>
        <ol className="network-steps">{([
            ['dmit','DMIT','获取 VPS'],['webshare','Webshare','获取家宽'],['skill','Skill','安装 Skill']
        ] as const).map(([provider,name,label],index)=><li className="network-step" key={provider}>
            <div className="network-step-heading"><span className="network-step-number" aria-hidden="true">{String(index+1).padStart(2,'0')}</span><h2>{name}</h2></div>
            <div className="network-step-actions"><Button variant={provider==='dmit'?'info':provider==='webshare'?'success':'secondary'} size="small" leftIcon={provider==='dmit'?<ServerIcon size={15} aria-hidden="true"/>:provider==='webshare'?<HomeIcon size={15} aria-hidden="true"/>:undefined} onClick={()=>void open(provider)}>{t(label)}</Button>{provider!=='skill'&&<Button variant="secondary" size="small" round aria-label={t(provider==='dmit'?'VPS 帮助':'家宽帮助')} onClick={()=>setGuide(provider)}>?</Button>}</div>
        </li>)}</ol>
        <div className="landing-section-band" aria-hidden="true"/>
        <section className="skill-install" aria-labelledby="skill-install-title">
            <h2 id="skill-install-title">{t('Skill')}</h2>
            <Tabs className="coding-tabs" size="small" variant="segmented" value={selected} onValueChange={setSelected} tabs={agents.map(agent=>({value:agent.id,label:<span className="coding-agent"><img src={agent.icon} width={18} height={18} alt=""/>{agent.id==='generic'?t('通用'):agent.name}</span>}))}/>
            <div className="skill-prompt-surface"><div className="skill-prompt-toolbar"><span>{selected==='generic'?t('通用'):agents.find(agent=>agent.id===selected)?.name}</span><span>{locale.toUpperCase()}</span></div>
                <Textarea id="skill-prompt" aria-labelledby="skill-install-title" readOnly value={prompt} rows={3}/>
                <div className="skill-prompt-actions"><Button variant="ink" disabled={source===undefined} onClick={()=>void copy()}>{copied===prompt?t('已复制'):t('复制提示词')}</Button></div>
            </div>
        </section>
        <AppDialog open={guide!==null} onOpenChange={value=>{if(!value)setGuide(null);}} title={guide==='dmit'?'DMIT':'Webshare'} width={440} footer={<Button variant="ink" onClick={()=>setGuide(null)}>{t('关闭')}</Button>}>
            <dl className="resource-specs">{(guide==='dmit'?[
                ['系统','Debian 12 / 13'],['地址','IPv4'],['登录','SSH Key · root:22'],['文件','PEM · PPK · PUB'],['密钥选项','Generate SSH Key after order']
            ]:[
                ['类型','Static Residential (ISP)'],['连接','Direct Connection'],['认证','Username / Password'],['协议','SOCKS5'],['端口','以面板为准']
            ]).map(([label,value])=><div key={label}><dt>{t(label)}</dt><dd>{t(value)}</dd></div>)}</dl>
        </AppDialog>
        {error&&<p role="alert" className="system-feedback error">{t(error)}</p>}
    </section>;
}
