| Placeholder | Value |
|---|---|
| `{{.Repo}}` | `owner/name` of the repository |
| `{{.Workflow}}` | Workflow name |
| `{{.Job}}` | Job id inside the workflow |
| `{{.Status}}` | The reported status, verbatim |
| `{{.StatusEmoji}}` | ✅ / ❌ / ⚠️ / ⏭ for the reported status |
| `{{.Actor}}` | User that triggered the run (git `user.name` outside Actions) |
| `{{.Ref}}` | Full ref, e.g. `refs/heads/main` |
| `{{.RefName}}` | Short ref name, e.g. `main` |
| `{{.Branch}}` | Alias of `RefName` |
| `{{.Sha}}` | Full commit SHA |
| `{{.ShortSha}}` | First 7 characters of the SHA |
| `{{.RunId}}` | Workflow run id (empty outside Actions) |
| `{{.RunNumber}}` | Workflow run number (empty outside Actions) |
| `{{.RunUrl}}` | Direct link to the run (empty outside Actions) |
| `{{.EventName}}` | Event that triggered the run (empty outside Actions) |
| `{{.ServerUrl}}` | GitHub server URL |
