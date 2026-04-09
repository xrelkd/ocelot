## Context

The Ocelot project has four main subcommands: `idle`, `entry`, `supervise`, and `bootstrap`. The README.md already documents `idle`, `entry`, `supervise`, and `zombie`, but is missing documentation for `bootstrap`. Additionally, the Configuration Reference section only covers the `supervise` command, but `bootstrap` has its own distinct configuration schema.

## Goals / Non-Goals

**Goals:**

- Add bootstrap command documentation to README.md Usage section (after supervise)
- Add bootstrap CLI reference to Command Line Interface section
- Split Configuration Reference into supervise and bootstrap with brief overview and links to full documentation
- Create separate configuration markdown files for supervise and bootstrap

**Non-Goals:**

- Not creating new code or API changes
- Not changing existing supervise functionality
- Not creating QEMU documentation (out of scope for this change)

## Decisions

1. **Where to place bootstrap in Usage section**: Place after supervise section, following the existing order (idle → entry → supervise → bootstrap → zombie)

2. **Configuration sections split approach**: Update README.md to have brief overview with links to:
   - `docs/supervise-config.md` - Full supervise config documentation (extracted from README)
   - `docs/bootstrap-config.md` - Full bootstrap config documentation

3. **Brief config intro in README**: Add 2-3 sentence introduction explaining the difference between supervise and bootstrap configs, guiding users to appropriate documentation

4. **Unsupported features**: The following features are in the codebase but NOT yet fully supported. They SHALL NOT be mentioned in README.md or configuration documentation:
   - Network configuration (`network` field)
   - Security module configuration (`security` field - SELinux, AppArmor)
   - Clock configuration (`clock` field)

## Risks / Trade-offs

- **Risk**: Users may expect inline configuration docs instead of linked files → Mitigation: Provide comprehensive links and brief overview in README
- **Risk**: Bootstrap config may change frequently → Mitigation: Keep config docs in separate files for easier maintenance
