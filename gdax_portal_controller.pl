#!/usr/bin/perl
# This program forks children to handle a number of slow tasks.  It
# uses POE::Filter::Reference so the child tasks can send back
# arbitrary Perl data.  The constant MAX_CONCURRENT_TASKS limits the
# number of forked processes that can run at any given time.
use warnings;
use strict;
use POE qw(Wheel::Run Filter::Reference);

# Start the session that will manage all the children.  The _start and
# next_task events are handled by the same function.
POE::Session->create(
    inline_states => {
        _start      => \&start_tasks,
        next_task   => \&start_tasks,
        task_result => \&handle_task_result,
        task_done   => \&handle_task_done,
        task_debug  => \&handle_task_debug,
        sig_child   => \&sig_child,
    }
);

sub start_tasks {
    my ($kernel, $heap) = @_[KERNEL, HEAP];

    my $task = POE::Wheel::Run->new(
        Program      => 'sh -c gdax_portal/run_release',
        StdoutFilter => POE::Filter::Line->new(),
        StdoutEvent  => "task_result",
        StderrEvent  => "task_debug",
        CloseEvent   => "task_done",
    );

    $heap->{task}->{$task->ID} = $task;
    $kernel->sig_child($task->PID, "sig_child");
}


# Handle information returned from the task.  Since we're using
# POE::Filter::Reference, the $result is however it was created in the
# child process.  In this sample, it's a hash reference.
sub handle_task_result {
    my $result = $_[ARG0];
    print "Result for $result->{task}: $result->{status}\n";
}

# Catch and display information from the child's STDERR.  This was
# useful for debugging since the child's warnings and errors were not
# being displayed otherwise.
sub handle_task_debug {
    my $result = $_[ARG0];
    print "Debug: $result\n";
}

# The task is done.  Delete the child wheel, and try to start a new
# task to take its place.
sub handle_task_done {
    my ($kernel, $heap, $task_id) = @_[KERNEL, HEAP, ARG0];
    delete $heap->{task}->{$task_id};
    $kernel->yield("next_task");
}

# Detect the CHLD signal as each of our children exits.
sub sig_child {
  my ($heap, $sig, $pid, $exit_val) = @_[HEAP, ARG0, ARG1, ARG2];
  my $details = delete $heap->{$pid};

  # warn "$$: Child $pid exited";
}

# Run until there are no more tasks.
$poe_kernel->run();
exit 0;