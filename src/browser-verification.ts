/**
 * [INPUT]: Native machine/context and separately measured browser observations.
 * [OUTPUT]: Strict cross-platform verification; unknown platforms/location never pass.
 * [POS]: Pure predicate shared by the start page and its regression fixtures.
 */
import type {BrowserContext, Machine, Report} from './browser-model';

export function hardwareMatches(machine: Machine, report: Report): boolean {
    const platform = machine.platform === 'windows'
        ? ['Win32', 'Win64', 'Windows'].includes(report.platform)
        : machine.platform === 'linux'
            ? report.platform.toLowerCase().includes('linux')
            : machine.platform === 'macos'
                ? ['MacIntel', 'macOS'].includes(report.platform)
                : false;
    return platform && report.cores === machine.cores
        && (machine.platform === 'linux' || report.platform_version === machine.version);
}

export function measuredEnvironmentMatches(context: BrowserContext, report: Report): boolean {
    const {machine, region, config} = context;
    if (!region || !report.exit || report.error) return false;
    const language = machine.languages[0];
    return report.exit.country === region.country
        && report.timezone === report.exit.timezone
        && report.timezone === machine.timezone
        && report.timezone === (config.follow_machine ? machine.timezone : config.timezone)
        && report.language === region.language
        && hardwareMatches(machine, report)
        && report.browser_version === context.engine_version
        && report.geolocation === 'denied'
        && machine.location_enabled === false
        && machine.locale.startsWith(region.locale)
        && (language === region.language || Boolean(language?.startsWith(`${region.language}-`)));
}
