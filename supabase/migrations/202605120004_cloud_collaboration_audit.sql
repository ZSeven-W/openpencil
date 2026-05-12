-- OpenPencil collaboration audit permissions, activity pagination indexes, and shared version controls.
-- Forward-only migration for Supabase Cloud.

create index if not exists activity_events_file_type_created_idx
  on public.activity_events(file_id, type, created_at desc)
  where file_id is not null;

drop policy if exists "activity events are visible to owners and actors" on public.activity_events;
drop policy if exists "activity events are visible to owners actors and shared recipients" on public.activity_events;
create policy "activity events are visible to owners actors and shared recipients"
on public.activity_events
for select
using (
  owner_id = auth.uid()
  or actor_id = auth.uid()
  or (
    file_id is not null
    and exists (
      select 1
      from public.design_file_shares dfs
      where dfs.file_id = activity_events.file_id
        and dfs.owner_id = activity_events.owner_id
        and dfs.revoked_at is null
        and (
          dfs.shared_with_user_id = auth.uid()
          or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
        )
    )
  )
);

drop policy if exists "editor shared file versions are readable" on public.design_file_versions;
drop policy if exists "shared file versions are readable by recipients" on public.design_file_versions;
create policy "shared file versions are readable by recipients"
on public.design_file_versions
for select
using (
  exists (
    select 1
    from public.design_file_shares dfs
    where dfs.file_id = design_file_versions.file_id
      and dfs.owner_id = design_file_versions.owner_id
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
);

drop policy if exists "editor shared file versions are label editable" on public.design_file_versions;
create policy "editor shared file versions are label editable"
on public.design_file_versions
for update
using (
  exists (
    select 1
    from public.design_file_shares dfs
    where dfs.file_id = design_file_versions.file_id
      and dfs.owner_id = design_file_versions.owner_id
      and dfs.role = 'editor'
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
)
with check (
  exists (
    select 1
    from public.design_file_shares dfs
    where dfs.file_id = design_file_versions.file_id
      and dfs.owner_id = design_file_versions.owner_id
      and dfs.role = 'editor'
      and dfs.revoked_at is null
      and (
        dfs.shared_with_user_id = auth.uid()
        or lower(coalesce(dfs.shared_with_email, '')) = lower(coalesce(auth.jwt() ->> 'email', ''))
      )
  )
);
