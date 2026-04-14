import { invoke } from '@tauri-apps/api/core';

export interface TestCase {
	id: string;
	name: string;
	description: string;
	category: string;
	tags: string;
	content: string;
	expected_message_type: string;
	expected_validation_result: string;
	created_at: string;
	updated_at: string;
}

export async function saveTestCase(
	testCase: Partial<TestCase> & {
		name: string;
		content: string;
	}
): Promise<TestCase> {
	return invoke('save_test_case', {
		id: testCase.id ?? null,
		name: testCase.name,
		description: testCase.description ?? '',
		category: testCase.category ?? 'general',
		tags: testCase.tags ?? '',
		content: testCase.content,
		expectedMessageType: testCase.expected_message_type ?? '',
		expectedValidationResult: testCase.expected_validation_result ?? 'valid',
	});
}

export async function getTestCases(category?: string): Promise<TestCase[]> {
	return invoke('get_test_cases', { category: category ?? null });
}

export async function deleteTestCase(id: string): Promise<void> {
	return invoke('delete_test_case', { id });
}
