# Security Policy

## Supported Versions

We release patches for security vulnerabilities. Which versions are currently supported with security updates depends on the active development cycle:

| Version | Supported          |
| ------- | ------------------ |
| Latest (`main`) | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability within Rune, please report it responsibly. **Do not disclose security vulnerabilities publicly until they have been addressed and a patch has been released.**

To report a vulnerability, please use one of the following methods:

- **Private Vulnerability Disclosure**: Use GitHub's private vulnerability reporting feature on the repository (if enabled), or contact the maintainers directly.
- **Email**: Reach out directly to the primary maintainer (`SickleFire`) via GitHub profile contact information.

Please include as much detail as possible in your report to help us reproduce and fix the issue quickly:
- Type of vulnerability (e.g., path traversal, remote code execution, unauthorized access).
- Full paths of source file(s) related to the vulnerability.
- Step-by-step instructions to reproduce the issue.
- Proof-of-concept or exploit code (if applicable).
- Potential remediation ideas or patches.

## Response Process

1. **Acknowledgment**: We will acknowledge receipt of your vulnerability report within 48 hours.
2. **Investigation**: We will investigate the issue, verify its impact, and assess affected components (such as file system tools, web tools, or editor bridges).
3. **Patching**: We will develop, test, and review a fix privately.
4. **Coordinated Disclosure**: Once a patch is released, we will credit the reporter (unless requested otherwise) and publish advisories as appropriate.

Thank you for helping keep Rune and our community safe!
