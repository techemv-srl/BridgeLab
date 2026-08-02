import type { ManualSection } from '../helpContent';
import { getStarted, editorSection, treeSection } from './fr-part1';
import { validationSection, communicationSection, anonymizationSection, testCasesSection } from './fr-part2';
import { schemaExportSection, fhirSection, pluginsSection, licensingSection, shortcutsSection } from './fr-part3';

export const frSections: ManualSection[] = [
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
