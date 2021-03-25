#!/usr/bin/perl
# This program forks children to handle a number of slow tasks.  It
# uses POE::Filter::Reference so the child tasks can send back
# arbitrary Perl data.  The constant MAX_CONCURRENT_TASKS limits the
# number of forked processes that can run at any given time.
use warnings;
use strict;
use POE qw(Wheel::Run Filter::Reference Wheel::ReadWrite);

use v5.32.0;

# Start the session that will manage all the children.  The _start and
# next_task events are handled by the same function.
POE::Session->create(
    inline_states => {
        _start              =>  \&start,
        new_worker          =>  \&new_worker,
        task_result         =>  \&handle_task_result,
        task_stdout         =>  \&handle_task_stdout,
        task_stderr         =>  \&handle_task_stderr,
        process_market_data =>  \&process_market_data,
        sig_child           =>  \&sig_child,
    },
    heap => {
    }
);

sub start {
    my ($kernel,$heap) =  @_[KERNEL, HEAP];
    $kernel->yield('new_worker');
    $kernel->delay_add('new_worker' => 5);
    $kernel->delay_add('new_worker' => 10);
    $kernel->delay_add('new_worker' => 15);
}

sub new_worker {
    my ($kernel, $heap) = @_[KERNEL, HEAP];

    my $task = POE::Wheel::Run->new(
        Program      => 'sh -c gdax_portal/run_release',
        StdoutFilter => POE::Filter::Line->new(),
        StdoutEvent  => "task_stdout",
        StderrEvent  => "task_stderr",
        CloseEvent   => "task_done",
    );

    $heap->{task}->{$task->ID} = { obj=>$task, pid=>$task->PID, id=>$task->ID, rx=>0 };
    $heap->{task}->{$task->PID} = $task->ID;

    $kernel->sig_child($task->PID, "sig_child");
}


# Handle information returned from the task.  Since we're using
# POE::Filter::Reference, the $result is however it was created in the
# child process.  In this sample, it's a hash reference.
sub handle_task_stdout {
    my ($kernel,$heap,$stdout,$wheel_id) = @_[KERNEL,HEAP,ARG0,ARG1];

    # Increment the rc count for the wheel
    $heap->{task}->{$wheel_id}->{rx}++;
    if ($heap->{task}->{$wheel_id}->{rx} % 100000 == 0) {
        say "worker($wheel_id): ".$heap->{task}->{$wheel_id}->{rx}." processed records.";
    }

    my @dataset = split(/\s+/,$stdout,5);

    if (defined($dataset[3]) && $dataset[2] eq 'ERROR') {
        my $pid = $heap->{task}->{$wheel_id}->{pid};
        kill 1,$pid;
        say "worker($wheel_id) Kill";
    }
    else {
        say "($wheel_id): $stdout";
    }
}

# Catch and display information from the child's STDERR.  This was
# useful for debugging since the child's warnings and errors were not
# being displayed otherwise.
sub handle_task_stderr {
    my ($kernel,$heap,$result,$wheel_id) = @_[KERNEL,HEAP,ARG0,ARG1];
    print "Debug($wheel_id): $result\n";
}

sub handle_task_result {
    my ($kernel,$heap,$result,$wheel_id) = @_[KERNEL,HEAP,ARG0,ARG1];
    say "Task ended($wheel_id), restarting";

    my $pid = delete $heap->{task}->{$wheel_id}->{pid};
    delete $heap->{task}->{$pid};

    $kernel->yield('new_worker');
}

# Detect the CHLD signal as each of our children exits.
sub sig_child {
    my ($kernel,$heap, $sig, $pid, $exit_val) = @_[KERNEL,HEAP, ARG0, ARG1, ARG2];

    my $wid = delete $heap->{task}->{$pid};
    my $rxc = $heap->{task}->{$wid}->{rx};
    delete $heap->{task}->{$wid};

    say "Sig($wid) received on worker, pid $pid (rx: $rxc)";

    $kernel->yield('new_worker');
}

# Run until there are no more tasks.
$poe_kernel->run();
exit 0;