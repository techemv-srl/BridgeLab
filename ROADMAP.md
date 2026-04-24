# BridgeLab Roadmap 2026-2028

Strategic development plan based on competitor analysis, community needs research, and market study (April 2026).

---

## 1. Executive Summary

**Position**: BridgeLab is a modern HL7/FHIR desktop editor targeting the underserved middle of the market - more capable than free tools (HL7 Inspector, HAPI TestPanel), more affordable than enterprise ($8.7k/year HL7 Spy, $25-100k Iguana).

**Key findings from research**:
- **7edit is obsolete** (last meaningful update ~2015, Windows-only, unsupported)
- **Mirth Connect went closed-source March 2025** (v4.6+), orphaning thousands of teams
- **No unified HL7 v2 + FHIR editor** with equal depth for both standards
- **FHIR adoption at 71%** of countries (2025) but 78% have FHIR mandates - growing fast
- **Forge/Simplifier pricing shifted upmarket June 2025**, creating FHIR editor gap

**Strategic window**: 2026-2027 is the moment to establish market position before Mirth forks (OIE, BridgeLink) mature or incumbents move downmarket.

**Target revenue trajectory**:
- Year 1 (2026): $362K ARR (freemium + small teams)
- Year 2 (2027): $1.5-2M ARR (enterprise pilots + TEFCA pull-through)
- Year 3 (2028): $5M+ ARR (platform extensions, partnerships)

---

## 2. Competitor Landscape

| Product | Pricing | Status | Key Weakness |
|---------|---------|--------|--------------|
| **7edit** | ~$300-500/seat | Stalled (~2015) | No FHIR, dated UI, Windows-only |
| **HL7 Spy** | ~$8,745/year | Active | Expensive, Windows-only, no FHIR edit |
| **HL7 Inspector** | Free (GPL) | Stalled (2020s) | No FHIR, minimal UI |
| **HAPI TestPanel** | Free (MPL) | Low activity | No FHIR, outdated |
| **Chameleon** | Bundled Iguana | Active | Not standalone, dev-only API |
| **Iguana** | $25-75K/year | Very active | Integration engine, steep learning curve |
| **Rhapsody** | Enterprise custom | KLAS #1 (2024-26) | Enterprise-only, high cost |
| **Mirth 4.5.2** | Free (legacy) | Frozen | Fragmented post-licensing change |
| **Mirth 4.6+** | Commercial | Active | Pricing opacity, users leaving |
| **OIE / BridgeLink** | Free (forks) | Nascent (2025) | Unproven, small community |
| **Forge (Firely)** | ~$60+/month | Active | FHIR-only, expensive for small teams |
| **Simplifier** | Freemium | Active | Web-only, free tier restrictive |

### Key Vulnerabilities to Exploit

1. **Price gap**: Nothing between "free but abandoned" and "$8.7K/year"
2. **Unified v2+FHIR**: No leader owns both standards
3. **Modern UX**: All HL7 v2 tools feel 2010s
4. **Mirth orphans**: Mirth 4.5.2 users need a new home for message editing
5. **Forge exodus**: Firely moved Forge off Entry plan June 2025, pushing FHIR modelers to find alternatives

---

## 3. Community Unmet Needs (Prioritized)

Based on Reddit, HL7 Confluence, FHIR Chat, Stack Overflow, GitHub issues analysis.

### Priority 1 - Critical gaps (high demand, no good solution)

| # | Need | Demand | BridgeLab Difficulty |
|---|------|--------|----------------------|
| 1 | Interactive FHIR Bundle visualizer with reference navigation | Very high | Medium |
| 2 | Modern web/desktop HL7 editor (7edit replacement) | Very high | **Already built** |
| 3 | Team collaboration on message testing (Git-like workflow) | High | Medium |
| 4 | Large message handling (5-10MB base64) | High | **Already built** |

### Priority 2 - High-value pain points

| # | Need | Demand | BridgeLab Difficulty |
|---|------|--------|----------------------|
| 5 | HL7 v2 → FHIR conversion with incremental migration | High | Hard |
| 6 | De-identification / PHI anonymization workflows | High | **Already built (extend)** |
| 7 | Custom FHIR validation rule authoring (FHIRPath) | Medium | Medium-Hard |

### Priority 3 - Important incremental

| # | Need | Demand | BridgeLab Difficulty |
|---|------|--------|----------------------|
| 8 | HIPAA/GDPR audit trails & reports | Medium | Medium |
| 9 | Web-based composition tool (no install) | Medium | Easy |
| 10 | CI/CD integration for healthcare testing | Medium | Medium |
| 11 | Realistic test data generator | Low-Medium | Easy |
| 12 | FHIR Implementation Guide authoring | Low (niche) | Very Hard |

### Immediate competitive advantages BridgeLab has

BridgeLab already ships features that are missing or weak in competitors:
- Modern UI (Svelte 5 + Monaco)
- Native HL7 v2 + FHIR parsing
- 10MB+ message handling with smart truncation
- Anonymization engine (21 PHI fields)
- 5-language i18n (EN, IT, FR, ES, DE)
- MLLP + HTTP transport with troubleshooting params
- Offline Ed25519 license verification
- Cross-platform (Win + macOS Intel/ARM + Linux)
- **XSD schema export** for HL7 v2 message types (closes a long-standing 7edit-era gap; 4 common messages free, full catalogue Pro)

---

## 4. Development Roadmap 2026-2028

### Q2 2026 - Polish & Launch (v0.1 → v1.0)

**Focus**: stabilize core, release to public, build initial community.

- [ ] Beta testing program (50-100 users)
- [ ] Fix critical bugs from field testing
- [ ] Polish: keyboard shortcut customization, theme editor
- [x] **XSD schema export** — Tools → Export message schema as XSD…; covers ADT^A01 / ADT^A40 / ORM^O01 / ORU^R01 in v2.5 (Community), full message catalogue and other versions gated behind Pro
- [ ] Full v2.5 message catalogue via `hl7-schema-importer` (data-driven schema loader in place; importer for `hl7-dictionary` source pending)
- [ ] Additional HL7 versions (v2.3, v2.4, v2.6, v2.7, v2.8) via the same importer
- [ ] Open-source the HL7 parser core as separate crate on crates.io
- [ ] Launch on GitHub, Product Hunt, Hacker News, Reddit r/healthIT
- [ ] Documentation site (bridgelab.dev)
- [ ] Video tutorials (5-10 short clips)
- [ ] v1.0 release

### Q3 2026 - FHIR Deep Dive (v1.1)

**Focus**: become the best FHIR editor available, close Forge gap.

- [ ] **Interactive FHIR Bundle visualizer** (P1 #1)
  - Click references to navigate contained resources
  - Graph view of Bundle dependencies
  - Inline resource inspector
- [ ] FHIRPath evaluator panel
- [ ] FHIR profile validation (via HL7 official validator)
- [ ] Custom FHIR validation rules builder (P2 #7)
- [ ] FHIR resource templates (Patient, Observation, Bundle examples)

### Q4 2026 - Integration & Testing (v1.2)

**Focus**: developer workflow, CI/CD enablement.

- [ ] **CLI tool `bridgelab-cli`** - headless validation for CI
- [ ] Test case library (store reusable HL7/FHIR scenarios)
- [ ] Git integration (save messages to repo, diff across commits)
- [ ] SOAP client with WSDL parsing (Enterprise feature)
- [ ] Message generator with realistic fake data (P3 #11)
- [ ] Batch operations (validate/transform 1000+ messages)

### Q1 2027 - Collaboration (v1.3)

**Focus**: team workflows, shared libraries.

- [ ] **Team Workspaces** (Pro/Enterprise feature)
  - Shared test case libraries
  - Commenting on specific message fields
  - Review/approval workflows
- [ ] **HL7 v2 → FHIR converter** (P2 #5)
  - Uses official HL7 v2-to-FHIR implementation guide
  - Visual mapping editor
  - Round-trip testing
- [ ] HIPAA audit trail logging (P3 #8)
- [ ] Anonymization rules editor (extend beyond 21 built-ins)

### Q2-Q3 2027 - Platform Extensions (v2.0)

**Focus**: platform plays, partnerships.

- [ ] Plugin system for third-party extensions
- [ ] Marketplace for community plugins (validation packs, custom exports)
- [ ] Cloud sync for settings & templates (optional, E2E encrypted)
- [ ] Command-line deployment (Docker image)
- [ ] Mirth Connect message importer (win over orphaned Mirth users)
- [ ] Web-based lite version (P3 #9) for quick access

### Q4 2027 - Enterprise Features (v2.1)

**Focus**: close enterprise deals, compliance.

- [ ] SAML/SSO support
- [ ] Role-based access control
- [ ] SOC2 Type II certification
- [ ] HIPAA BAA templates
- [ ] Advanced audit logging
- [ ] Priority support SLA portal

### 2028 - AI-Assisted Features (v3.0)

- [ ] AI-assisted HL7 v2 → FHIR mapping (suggest mappings)
- [ ] Natural language query over messages ("find all PIDs where DOB < 1950")
- [ ] Anomaly detection on message streams
- [ ] Automated documentation generation for interfaces

---

## 5. Pricing Strategy

### Proposed Tiers (annual, per-seat)

| Tier | Price | Target | Features |
|------|-------|--------|----------|
| **Free** | €0 | Students, non-commercial, hobbyists | Core HL7v2 + FHIR view, trial 30 days on Pro features |
| **Pro** | €299/year | Freelance integration consultants | MLLP, HTTP, anonymization, export, validation, templates |
| **Team** | €1,499/year (5 seats) | Hospital IT teams, consulting firms | + Team workspaces, shared libraries, priority support |
| **Enterprise** | €15K-50K/year (custom) | Health systems, vendors, HIEs | + SSO, audit trails, HIPAA BAA, dedicated account manager |

### Rationale

- **Undercut HL7 Spy by 20-30x on Pro tier** - captures freelancer segment (~300K worldwide)
- **Team tier matches consulting firm budgets** without reaching Iguana pricing
- **Enterprise competes with Rhapsody ecosystem** at 10-30% of their cost
- **Free tier drives GitHub stars and community growth**

### Revenue Projection

| Year | Pro conversions | Team customers | Enterprise | ARR |
|------|-----------------|----------------|------------|-----|
| 2026 | 500 × €299 | 5 × €1,499 | 2 × €30K | ~€220K |
| 2027 | 2,000 × €299 | 30 × €1,499 | 10 × €30K | ~€940K |
| 2028 | 5,000 × €299 | 100 × €1,499 | 30 × €30K | ~€2.5M |

### Discounts & Free Access

- **Academic**: 100% free for students and educators (letter of enrollment required)
- **Open source projects**: Free Pro tier for maintainers
- **Non-profit healthcare**: 50% discount
- **HL7 community volunteers**: Free Enterprise access for HL7 working group members

---

## 6. Distribution & Go-to-Market

### Year 1 (2026) - Awareness

1. **Open source parser** on GitHub (MIT) - drive stars, establish credibility
2. **Launch posts** - Hacker News, Reddit r/healthIT / r/fhir, Product Hunt
3. **Content marketing** - 1 blog post/week on HL7 debugging, FHIR migration, tool comparisons
4. **Community engagement** - answer on Stack Overflow, FHIR Chat, HL7 Confluence
5. **HIMSS 2026 booth** (small, ~€10K investment)
6. **FHIR DevDays booth** (June 2026, Minneapolis)

### Year 2 (2027) - Growth

1. **HL7 Connectathons** - sponsor events, free licenses for participants
2. **Partnerships** - Mirth consultants as referral partners (earn 20% first year)
3. **Case studies** - 3-5 hospital pilots published as evidence
4. **Webinar series** - "Modern HL7 debugging" every 6 weeks
5. **YouTube channel** - tutorial videos, integration workflows
6. **Enterprise outreach** - targeted LinkedIn campaigns to CIOs/CIIOs

### Year 3 (2028) - Platform

1. **Marketplace launch** for plugins
2. **Certification program** - "BridgeLab Certified Integration Engineer" paid course
3. **Books/guides** - "The BridgeLab Handbook" (free PDF, paid print)
4. **EHR partnerships** - Epic App Orchard, Cerner marketplace listings

---

## 7. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Mirth/OIE becomes free dominant alternative | High | Focus on UX + FHIR + compliance features OIE won't have |
| Forge reverses pricing change, recaptures FHIR users | Medium | Build features Forge lacks (v2, MLLP, anonymization) |
| HL7 Spy adds FHIR + drops price | Medium | Our modern stack + cross-platform is hard to replicate |
| Enterprises see us as "too new" | High | SOC2 + case studies + open-source parser = credibility |
| Piracy of license keys | Low | Ed25519 signatures + hardware binding + call-home (optional) |
| Regulatory changes reduce HL7 v2 demand | Low (5-10 year) | Dual-standard support; v2 won't disappear before 2035+ |

---

## 8. Success Metrics

### Year 1 targets (end of 2026)
- GitHub stars on parser: 500+
- Free tier users: 10,000
- Paid customers: 500+
- ARR: €200K+
- NPS: >40

### Year 2 targets (end of 2027)
- GitHub stars: 2,500+
- Free users: 50,000
- Paid customers: 2,000+
- ARR: €900K+
- 10 enterprise accounts
- Mentioned in 2+ industry reports (KLAS, Gartner)

### Year 3 targets (end of 2028)
- GitHub stars: 5,000+
- Paid customers: 5,000+
- ARR: €2.5M+
- 30 enterprise accounts
- Plugin marketplace with 20+ community plugins
- Regional expansion (DACH, LATAM)

---

## 9. Immediate Next Steps (Post-Roadmap Approval)

1. **Week 1-2**: Fix known issues from beta testing, polish v0.1 → v0.2
2. **Week 3-4**: Setup documentation site, marketing landing page
3. **Week 5-6**: Release open-source HL7 parser crate on crates.io
4. **Week 7-8**: Launch beta publicly (Reddit, HN, PH)
5. **Week 9+**: Iterate based on feedback, plan Q3 FHIR Deep Dive

---

*Document version 1.0 - April 2026 - Based on market research agents*

