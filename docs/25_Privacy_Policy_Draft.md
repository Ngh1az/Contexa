# Privacy Policy (Draft)

**Project:** Contexa — AI Context Platform  
**Version:** 1.0 (Draft — requires legal review)  
**Status:** Draft — Legal Review Required  
**Last Updated:** 2026-07-06  
**Effective Date:** [TBD at GA launch]

---

> **Disclaimer:** This is a technical draft for product development alignment. It must be reviewed and approved by qualified legal counsel before publication.

---

## 1. Introduction

Contexa ("we", "us", "our") operates the Contexa desktop application and website (contexa.dev). This Privacy Policy explains how we collect, use, store, and protect your information.

**Core principle:** Your data stays on your device by default. We do not operate cloud servers that store your desktop context.

---

## 2. Information We Collect

### 2.1 Data Stored Locally on Your Device

| Data Type | Purpose | Retention |
|-----------|---------|-----------|
| Context snapshots | Build AI context from your active window | Per your retention setting (default 90 days) |
| Timeline events | Activity history and recall | Per your retention setting |
| Memory chunks | Semantic search over your work history | Per your retention setting |
| AI interaction history | Your queries and AI responses | Per your retention setting |
| Application settings | Your preferences and configuration | Until you delete |
| Exclusion lists | Apps/URLs you choose not to capture | Until you delete |

**This data never leaves your device unless you explicitly configure a cloud LLM provider or enable internet search.**

### 2.2 Data Sent to Third Parties (Only With Your Action)

| Third Party | Data Sent | When | Your Control |
|-------------|-----------|------|--------------|
| LLM provider (OpenAI, etc.) | Assembled prompt including context | When you initiate an AI action | Configure provider; use local LLM (Ollama) to avoid |
| Search provider (DuckDuckGo default, Brave opt-in) | Search query | When you search and search is enabled | Disabled by default; toggle in Settings |
| Update server | App version, OS version | Daily update check | Cannot disable updates; no personal data sent |

### 2.3 Data We Do NOT Collect

- Screenshots or screen recordings
- Keystrokes or input outside of captured window text
- Passwords (detected and redacted via UI Automation)
- Data from excluded applications
- Background analytics or telemetry (unless you opt in to crash reporting)
- Personal identification for cloud services (we have no user accounts in v1)

---

## 3. How We Use Your Information

| Use | Legal Basis (GDPR) |
|-----|-------------------|
| Build desktop context for AI features | Consent (onboarding) |
| Store timeline and memory for recall | Consent (onboarding) |
| Send prompts to your chosen LLM | Consent (per-action) |
| Search the internet on your behalf | Consent (opt-in setting) |
| Check for software updates | Legitimate interest |
| Crash reporting (opt-in) | Consent |

---

## 4. Data Storage and Security

- All data stored in `%APPDATA%\Contexa\contexa.db` on your local device
- API keys stored in Windows Credential Manager (not in database)
- MCP server accessible only on localhost with token authentication
- Database file protected by Windows file permissions (owner-only)
- No encryption at rest in v1.0; **SQLCipher available in v1.1 Pro tier**

---

## 5. Your Rights

| Right | How to Exercise |
|-------|-----------------|
| **Access** | Settings → Privacy → Export Data (JSON) |
| **Deletion** | Settings → Privacy → Delete All Data |
| **Rectification** | Edit or delete individual timeline entries |
| **Restriction** | Pause capture; add apps to exclusion list |
| **Portability** | Export data in JSON format |
| **Objection** | Disable capture; uninstall application |
| **Withdraw consent** | Delete all data and uninstall |

---

## 6. Data Retention

- Default retention: 90 days (configurable in Settings → Memory)
- Automatic purge of expired data during idle periods
- "Delete All Data" immediately removes all local data and credentials
- Uninstaller offers option to remove user data

---

## 7. MCP and Third-Party AI Clients

When you generate an MCP token and configure an external AI client (e.g., Cursor):

- The client can access your desktop context via localhost API
- You control which clients have access via token management
- All MCP access is logged in a local audit log
- You can revoke tokens at any time in Settings → MCP

---

## 8. Children's Privacy

Contexa is not intended for users under 16. We do not knowingly collect data from children.

---

## 9. International Users

Contexa processes all data locally on your device. No data is transferred internationally by Contexa. If you configure a cloud LLM provider, data transfer is between you and that provider under their privacy policy.

---

## 10. Changes to This Policy

We will notify you of material changes via the application update mechanism and post the revised policy at contexa.dev/privacy.

---

## 11. Contact

**Email:** privacy@contexa.dev  
**Address:** [Company address at incorporation]

---

## 12. References

- [16_Security_Privacy.md](./16_Security_Privacy.md)
- [01_Software_Requirements_Specification.md](./01_Software_Requirements_Specification.md)
