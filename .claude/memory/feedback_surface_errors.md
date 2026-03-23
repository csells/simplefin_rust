---
name: feedback_surface_errors
description: Never silently swallow API errors/messages — always surface them to the user
type: feedback
---

Never silently discard error messages, warnings, or informational messages from APIs. Always surface them to the user.

**Why:** During SimpleFIN integration, server_messages from the API were being completely ignored, hiding the fact that 11 accounts needed re-authentication. The user was told "everything works" when it didn't. This wasted significant debugging time and eroded trust.

**How to apply:** When working with any API response that includes status messages, warnings, or error fields — always display them. Don't assume "informational" means unimportant. If the API is telling us something, the user needs to see it.
