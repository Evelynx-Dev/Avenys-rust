; ModuleID = 'bin/release/main.ll'
source_filename = "bin/release/main.ll"

@.str0 = private unnamed_addr constant [12 x i8] c"str maybe? \00"

declare ptr @mire_i64_to_string(i64) local_unnamed_addr

declare ptr @mire_string_concat(ptr, ptr) local_unnamed_addr

define noundef i64 @mire_main() local_unnamed_addr {
entry:
  %t3 = tail call ptr @mire_i64_to_string(i64 10)
  %t6 = tail call ptr @mire_string_concat(ptr nonnull @.str0, ptr %t3)
  %puts = tail call i32 @puts(ptr nonnull dereferenceable(1) %t6)
  ret i64 0
}

define noundef i32 @main() local_unnamed_addr {
entry:
  %t3.i = tail call ptr @mire_i64_to_string(i64 10)
  %t6.i = tail call ptr @mire_string_concat(ptr nonnull @.str0, ptr %t3.i)
  %puts.i = tail call i32 @puts(ptr nonnull dereferenceable(1) %t6.i)
  ret i32 0
}

; Function Attrs: nofree nounwind
declare noundef i32 @puts(ptr noundef readonly captures(none)) local_unnamed_addr #0

attributes #0 = { nofree nounwind }
