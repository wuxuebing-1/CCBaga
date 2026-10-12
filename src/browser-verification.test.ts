/** [INPUT]: Synthetic cross-platform observations. [OUTPUT]: No false success after platform or measured state mismatch. */
import {expect, test} from 'bun:test';
import type {BrowserContext, Report} from './browser-model';
import {hardwareMatches, measuredEnvironmentMatches} from './browser-verification';

const context: BrowserContext = {
    ui_language:'en', exit_services:[], engine_version:'148.0.7778.215',
    region:{id:'us-east',label:'US',country:'US',language:'en-US',locale:'en_US',timezone:'America/New_York'},
    config:{seed:1,proxy:'',follow_machine:true,language:'en-US',timezone:'America/New_York',native_graphics:false},
    machine:{platform:'windows',version:'10.0.26200',cores:20,timezone:'America/New_York',languages:['en-US'],locale:'en_US',location_enabled:false,timezones:['America/New_York']},
};
const report: Report = {
    platform:'Win32',platform_version:'10.0.26200',cores:20,browser_version:'148.0.7778.215',user_agent:'fixture',
    language:'en-US',languages:['en-US'],timezone:'America/New_York',geolocation:'denied',error:'',
    exit:{ip:'203.0.113.1',country:'US',city:'New York',timezone:'America/New_York',isp:'fixture'},
};

test('Windows success requires actual Windows platform, native version and CPU',()=>{
    expect(measuredEnvironmentMatches(context,report)).toBe(true);
    for (const changed of [{platform:'MacIntel'}, {platform:'Linux x86_64'}, {platform_version:'15.0.0'}, {cores:8}]) {
        expect(measuredEnvironmentMatches(context,{...report,...changed})).toBe(false);
    }
    expect(hardwareMatches({...context.machine,platform:'other'},report)).toBe(false);
});

test('Unobserved location, connection error or wrong requested timezone cannot pass',()=>{
    expect(measuredEnvironmentMatches({...context,machine:{...context.machine,location_enabled:null}},report)).toBe(false);
    expect(measuredEnvironmentMatches(context,{...report,error:'HTTP 429'})).toBe(false);
    expect(measuredEnvironmentMatches(context,{...report,exit:null})).toBe(false);
    expect(measuredEnvironmentMatches({...context,config:{...context.config,follow_machine:false,timezone:'America/Los_Angeles'}},report)).toBe(false);
});

test('Existing macOS and Linux hardware rules remain intact',()=>{
    expect(hardwareMatches({...context.machine,platform:'macos'}, {...report,platform:'MacIntel'})).toBe(true);
    expect(hardwareMatches({...context.machine,platform:'linux'}, {...report,platform:'Linux x86_64',platform_version:''})).toBe(true);
});
