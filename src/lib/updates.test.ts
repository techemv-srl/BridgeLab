import { describe, it, expect } from 'vitest';
import { isNewerVersion, startupCheckDue, shouldNotify, effectivePreference, needsFirstRunQuestion, CHECK_INTERVAL_MS } from './updates';

describe('isNewerVersion', () => {
	it('compares numerically per part', () => {
		expect(isNewerVersion('1.10.0', '1.9.9')).toBe(true);
		expect(isNewerVersion('1.7.0', '1.7.0')).toBe(false);
		expect(isNewerVersion('1.6.9', '1.7.0')).toBe(false);
		expect(isNewerVersion('2.0', '1.99.99')).toBe(true);
	});
	it('ignores a leading v', () => {
		expect(isNewerVersion('v1.8.0', '1.7.0')).toBe(true);
	});
});

describe('startupCheckDue', () => {
	const now = 1_800_000_000_000;
	it('is on by default and when never checked', () => {
		expect(startupCheckDue(null, null, now)).toBe(true);
		expect(startupCheckDue('true', '', now)).toBe(true);
	});
	it('is off only for an explicit "false"', () => {
		expect(startupCheckDue('false', null, now)).toBe(false);
	});
	it('runs at most once a day', () => {
		expect(startupCheckDue(null, String(now - 60_000), now)).toBe(false);
		expect(startupCheckDue(null, String(now - CHECK_INTERVAL_MS), now)).toBe(true);
	});
	it('treats garbage or a future timestamp as never checked', () => {
		expect(startupCheckDue(null, 'yesterday', now)).toBe(true);
		expect(startupCheckDue(null, String(now + 3_600_000), now)).toBe(true);
	});
});

describe('shouldNotify', () => {
	it('notifies only for a newer, not skipped version', () => {
		expect(shouldNotify('1.8.0', '1.7.0', null)).toBe(true);
		expect(shouldNotify('1.8.0', '1.7.0', '1.8.0')).toBe(false);
		expect(shouldNotify('1.9.0', '1.7.0', '1.8.0')).toBe(true);
		expect(shouldNotify('1.7.0', '1.7.0', null)).toBe(false);
		expect(shouldNotify('1.8.0', '', null)).toBe(false);
	});
});

describe('effectivePreference', () => {
	const none = { disabled_by_policy: false, policy_source: null, installer_choice: null };
	it('lets a machine policy win over everything', () => {
		const policy = { ...none, disabled_by_policy: true, policy_source: '/etc/bridgelab/policy.json' };
		expect(effectivePreference(policy, 'true').value).toBe('false');
	});
	it('keeps the user choice over the installer choice', () => {
		expect(effectivePreference({ ...none, installer_choice: false }, 'true')).toEqual({ value: 'true', seedFromInstaller: false });
	});
	it('seeds from the installer the first time', () => {
		expect(effectivePreference({ ...none, installer_choice: false }, null)).toEqual({ value: 'false', seedFromInstaller: true });
	});
	it('falls back to the default (on) when nobody chose', () => {
		expect(effectivePreference(none, null)).toEqual({ value: null, seedFromInstaller: false });
		expect(effectivePreference(null, null).value).toBeNull();
	});
});

describe('needsFirstRunQuestion', () => {
	const none = { disabled_by_policy: false, policy_source: null, installer_choice: null };
	it('asks only when nobody has decided', () => {
		expect(needsFirstRunQuestion(none, null)).toBe(true);
		expect(needsFirstRunQuestion(null, null)).toBe(true);
	});
	it('does not ask after the user, the installer or a policy decided', () => {
		expect(needsFirstRunQuestion(none, 'true')).toBe(false);
		expect(needsFirstRunQuestion(none, 'false')).toBe(false);
		expect(needsFirstRunQuestion({ ...none, installer_choice: true }, null)).toBe(false);
		expect(needsFirstRunQuestion({ ...none, disabled_by_policy: true, policy_source: 'env' }, null)).toBe(false);
	});
});
