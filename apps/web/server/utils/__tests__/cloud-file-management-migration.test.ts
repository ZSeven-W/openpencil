import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const migration = readFileSync(
  join(process.cwd(), '../../supabase/migrations/202605120001_cloud_file_management.sql'),
  'utf-8',
).toLowerCase();
const sharingMigration = readFileSync(
  join(process.cwd(), '../../supabase/migrations/202605120002_cloud_file_sharing.sql'),
  'utf-8',
).toLowerCase();
const activityMigration = readFileSync(
  join(process.cwd(), '../../supabase/migrations/202605120003_cloud_version_activity.sql'),
  'utf-8',
).toLowerCase();
const collaborationAuditMigration = readFileSync(
  join(process.cwd(), '../../supabase/migrations/202605120004_cloud_collaboration_audit.sql'),
  'utf-8',
).toLowerCase();

describe('cloud file management migration', () => {
  it('adds project and folder containers for cloud files', () => {
    expect(migration).toContain('create table if not exists public.projects');
    expect(migration).toContain('create table if not exists public.folders');
    expect(migration).toContain('alter table public.design_files');
    expect(migration).toContain('add column if not exists project_id');
    expect(migration).toContain('add column if not exists folder_id');
  });

  it('adds metadata needed by the full cloud library views', () => {
    expect(migration).toContain('add column if not exists metadata jsonb');
    expect(migration).toContain('add column if not exists last_opened_at');
    expect(migration).toContain('add column if not exists starred boolean');
    expect(migration).toContain('deleted_at is not null');
  });

  it('keeps projects and folders protected by owner-scoped RLS', () => {
    expect(migration).toContain('alter table public.projects enable row level security');
    expect(migration).toContain('alter table public.folders enable row level security');
    expect(migration).toContain('owner_id = auth.uid()');
  });

  it('adds indexes for active files, recent files, starred files, trash, and name search', () => {
    expect(migration).toContain('design_files_project_folder_active_idx');
    expect(migration).toContain('design_files_owner_starred_idx');
    expect(migration).toContain('design_files_owner_recent_idx');
    expect(migration).toContain('design_files_owner_deleted_idx');
    expect(migration).toContain('design_files_owner_name_idx');
  });

  it('adds owner-managed cloud file sharing with recipient RLS', () => {
    expect(sharingMigration).toContain('create table if not exists public.design_file_shares');
    expect(sharingMigration).toContain("role text not null default 'viewer'");
    expect(sharingMigration).toContain('design_file_shares_owner_file_active_idx');
    expect(sharingMigration).toContain('shared_with_user_id = auth.uid()');
    expect(sharingMigration).toContain("auth.jwt() ->> 'email'");
    expect(sharingMigration).toContain('shared design files are readable');
    expect(sharingMigration).toContain('editor shared design files are editable');
  });

  it('adds version metadata and durable cloud activity events', () => {
    expect(activityMigration).toContain('add column if not exists actor_id');
    expect(activityMigration).toContain('add column if not exists size_bytes');
    expect(activityMigration).toContain('create table if not exists public.activity_events');
    expect(activityMigration).toContain('file_id uuid references public.design_files(id) on delete set null');
    expect(activityMigration).toContain('generation_id uuid references public.code_generations(id) on delete set null');
    expect(activityMigration).toContain("'file_force_saved'");
    expect(activityMigration).toContain("'codegen_created'");
    expect(activityMigration).toContain('activity events are visible to owners and actors');
    expect(activityMigration).toContain('activity events can be inserted by owners and shared editors');
    expect(activityMigration).toContain('actor_id = auth.uid()');
    expect(activityMigration).toContain('dfs.role = \'editor\'');
  });

  it('adds shared collaboration audit access and activity pagination indexes', () => {
    expect(collaborationAuditMigration).toContain('activity_events_file_type_created_idx');
    expect(collaborationAuditMigration).toContain(
      'activity events are visible to owners actors and shared recipients',
    );
    expect(collaborationAuditMigration).toContain('shared file versions are readable by recipients');
    expect(collaborationAuditMigration).toContain('editor shared file versions are label editable');
    expect(collaborationAuditMigration).toContain('dfs.role = \'editor\'');
    expect(collaborationAuditMigration).toContain("auth.jwt() ->> 'email'");
  });
});
