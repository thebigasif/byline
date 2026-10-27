### Added

- An Admin API edit schedules a draft with `status: scheduled` and a future `published_at`, schedules its removal with `unpublish_at`, and cancels with `status: draft`. A time that has passed, or an unpublish time before the publish time, is refused with a 422 that changes nothing (KH-API-2).
