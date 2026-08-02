import type { ManualSection } from '../helpContent';
import { getStarted, editorSection, treeSection } from './de-part1';
import { validationSection, communicationSection, anonymizationSection, testCasesSection } from './de-part2';
import { schemaExportSection, fhirSection, pluginsSection, licensingSection, shortcutsSection } from './de-part3';

export const deSections: ManualSection[] = [
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
