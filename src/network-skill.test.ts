/**
 * [INPUT]: 五项工具目录/调用入口与精简中英文 YAML 提示词。
 * [OUTPUT]: 目录、调用符、完整 Skill 来源、语言和恰好五项的回归。
 * [POS]: 网络页复制内容验收，不安装 Skill 或连接服务器。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {test,expect} from 'bun:test';
import {agents,installationPrompt} from './network-skill';

test('Windows 安装提示使用随包兼容 Skill，避免下载未移植的旧版本',()=>{
    for(const locale of ['zh','en'] as const){
        const prompt=installationPrompt('codex',locale,'C:\\Users\\Test User\\CCBaga-A\\network-skill');
        expect(prompt).toContain('network-skill');
        expect(prompt).toContain('Test User');
        expect(prompt).toContain('$ccbaga-network');
        expect(prompt).not.toContain('github.com');
    }
});
test('五项工具只复制正确语言的安装与运行提示词',()=>{
    expect(agents.map(a=>a.id)).toEqual(['generic','codex','cursor','opencode','copilot']);
    for(const agent of agents){
        const zh=installationPrompt(agent.id,'zh'),en=installationPrompt(agent.id,'en');
        if(agent.directory){expect(zh).toContain(agent.directory);expect(en).toContain(agent.directory);}
        expect(en).toContain('https://github.com/liseami/CCBaga');
        expect(en).toContain('skills/ccbaga-network');
        expect(en).toContain(agent.invoke);expect(zh).toContain(agent.invoke);
        expect(en).toContain('English');
        expect(/[\u3400-\u9fff]/.test(en)).toBe(false);
        expect(/[\u3400-\u9fff]/.test(zh)).toBe(true);
        expect(en).not.toContain('VPS');expect(en).not.toContain('TUN');
    }
});
