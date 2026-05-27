-- OpenPencil cloud file and notification performance indexes.
-- Optimizes cloud library, editor open, and global notification polling.

create index if not exists projects_owner_active_updated_idx
  on public.projects(owner_id, updated_at desc)
  where deleted_at is null;

create index if not exists projects_owner_active_created_idx
  on public.projects(owner_id, created_at asc)
  where deleted_at is null;

create index if not exists folders_owner_project_active_sort_idx
  on public.folders(owner_id, project_id, sort_order asc, name asc)
  where deleted_at is null;

create index if not exists folders_owner_project_parent_active_sort_idx
  on public.folders(owner_id, project_id, parent_id, sort_order asc, name asc)
  where deleted_at is null;

create index if not exists design_files_owner_project_root_updated_idx
  on public.design_files(owner_id, project_id, updated_at desc)
  where deleted_at is null and folder_id is null;

create index if not exists design_files_owner_project_folder_updated_idx
  on public.design_files(owner_id, project_id, folder_id, updated_at desc)
  where deleted_at is null;

create index if not exists design_files_owner_project_starred_updated_idx
  on public.design_files(owner_id, project_id, updated_at desc)
  where deleted_at is null and starred = true;

create index if not exists design_files_owner_project_recent_idx
  on public.design_files(owner_id, project_id, last_opened_at desc)
  where deleted_at is null and last_opened_at is not null;

create index if not exists design_files_owner_project_deleted_idx
  on public.design_files(owner_id, project_id, deleted_at desc)
  where deleted_at is not null;

create index if not exists task_notifications_owner_created_idx
  on public.task_notifications(owner_id, created_at desc);

create index if not exists codegen_worker_heartbeats_recent_idx
  on public.codegen_worker_heartbeats(last_heartbeat_at desc);

create index if not exists design_file_shares_user_active_idx
  on public.design_file_shares(shared_with_user_id, file_id)
  where revoked_at is null;

create index if not exists design_file_shares_email_active_idx
  on public.design_file_shares(lower(shared_with_email), file_id)
  where revoked_at is null and shared_with_email is not null;
