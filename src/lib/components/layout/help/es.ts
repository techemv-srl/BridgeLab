import type { ManualSection } from '../helpContent';
import { getStarted, editorSection, treeSection } from './es-part1';
import { validationSection, communicationSection, anonymizationSection, testCasesSection } from './es-part2';
import { schemaExportSection, fhirSection, pluginsSection, licensingSection, shortcutsSection } from './es-part3';

export const esSections: ManualSection[] = [
	getStarted,
	editorSection,
	treeSection,
	validationSection,
	communicationSection,
	anonymizationSection,
	testCasesSection,
	schemaExportSection,
	fhirSection,
	pluginsSection,
	licensingSection,
	shortcutsSection,
];
