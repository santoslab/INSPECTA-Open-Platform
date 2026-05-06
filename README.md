# INSPECTA Open Platform

## Open Platform Overview
  brief description of purpose of code-base; identification as part of INSPECTA project
  explanation that there are several variants of the artifacts provided

### Contents
- [ConOps and High-level System Architecture](#conops-and-high-level-system-architecture) 
- [Artifact Walkthrough](#artifact-walkthrough)
- [Tool Installation and Use](#tool-installation-and-use)
- [Key Papers, Tutorials, and Presentations](#key-papers-tutorials-and-presentations)


## ConOps and High-level System Architecture
  - describe the desire to illustrate aspects of securing a legacy autonomy system, with focus on information control flowing in/out of the autonomy application.  Point to the Collins Rapid Edge mission computer as the conceptual target.  Explain that Rapid Edge is proprietary so we can’t release that.  Open Platform is designed to illustrate INSPECTA capabilities similar to what will be used on hardening Rapid Edge
  - Mock system context 
       - explain mock scenario
            - UAV system (mocked by Open Platform)
            - ground station(s)
            - communication between ground stations and Open Platform, with high-level description of message contents between exchanged
  - Security Goals for UAV (high-level description of what we are trying to guard against) and Threat Model (i.e., what assumptions we are making)
  - PPT diagram describing system architecture, with supporting discussion
  - Role of seL4 in achieving security goals
  - Role of HAMR and broader INSPECTA tools in supporting development and assurance 

## Artifact Walkthrough
  - List of INSPECTA features that we aim to illustrate - with 2-3 sentence description of the purpose of each
    - System and component requirements 
    - SysMLv2 models, with AADL libraries
    - GUMBO specs and model-level verification
    - Resolint architectural rules (forth-coming)
    - HAMR code generation
    - Property-based testing of components
    - Verus verification of components
    - Vest specification and construction of message parser
    - Microkit seL4 specifications
    - LionsOS components

    - System Architecture Model
         - walkthrough of description of SysMLv2 architecture of system (with pointers to HAMR documentation)
         - summary of the purpose of each component, how is it realized, e.g., hardcoded Rust, Vest generated, VM 
         - overview of AADL data modeling used to define message formats
         - overview of SysMLv2/AADL used to specify kernel partitioning and threading

     - Application Components
          - and key INSPECTA technologies (provide summary, references, and links to detailed documentation for HAMR, Verus, PropTest, Vest, microkit, etc.)
          - Firewall 
                - hardcoded rust
                - link to examples of generated Verus contracts
                - link to tests (manual and property-based)
         - MavLink Firewall
               - link to Vest specs, 
               - link to generated artifacts, 
         - rest of components...

     - some sort of overview of CI, assurance and attestation artifacts as they become more mature.

## Tool Installation and Use
   - point to HAMR installation instructions for installing HAMR
   - add any additional requirements for supporting the Open Platform (running in qemu, etc.)

## Key Papers, Tutorials, and Presentations
  - Initial run of the open platform and observing logging messages etc.

